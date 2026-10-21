use std::env::var;
use std::future::Future;
use std::pin::Pin;
use sea_orm::DatabaseConnection;
use serde::{Serialize, Deserialize};
use actix_web::{FromRequest, dev::Payload, HttpRequest, HttpMessage, web};
use futures_util::future::{ready, Ready};
use jsonwebtoken::TokenData;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi, IntoResponses, ToSchema};

 

use crate::middleware::redis_cache::CacheReqResp;
use crate::schemas::request_sch::SortValues;
use crate::schemas::response_sch::ApiError;
use crate::utils::jwt::verify_jwt;


pub struct AppState{
    pub db: DatabaseConnection,
    pub redis: CacheReqResp,
    pub env_config: AppEnvConfig
}

#[derive(Clone)]
pub struct AppEnvConfig{
    pub db_url: String,
    pub redis_url: String,
    pub secret_key: String,
    pub port: u32,
    pub bind_address: String,
    pub cors_origin: String,
    pub rust_log_level: String
}

impl AppEnvConfig {
    pub fn from_env()-> Result<Self, anyhow::Error>{
        Ok(Self {
            db_url: var("DATABASE_URL").expect("Invalid database url ..."),
            redis_url: var("REDIS_URL").expect("Invalid redis url ..."),
            secret_key: var("SECRET_KEY").expect("Invalid secret key ..."),
            port: var("PORT_NUMBER").expect("Invalid Port number ...").parse::<u32>().expect("PORT NUMBER IS NOT A VALID U32 NUMBER"),
            bind_address: var("BIND_ADDRESS").expect("Invalid bind address ..."),
            cors_origin: var("CORS_ORIGIN").expect("Invalid CORS origin ..."),
            rust_log_level: var("RUST_LOG").unwrap_or("info".to_string()),
        })
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claims {
    pub sub: i64,
    pub iat: usize,
    pub exp: usize,
    pub email: String,
}

pub struct OptionalUser(pub Option<Claims>);

impl FromRequest for OptionalUser{
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_http::Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let state = req
                .app_data::<web::Data<AppState>>()
                .cloned()
                .ok_or_else(|| ApiError::NotFound("App State was not configured".to_string()))?;

            let token_header = req.cookie("access_token")
                .map(|c| c.value().to_string())
                .or_else(|| {
                    req
                        .headers()
                        .get("Authorization")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|h| h.strip_prefix("Bearer "))
                        .map(|s| s.to_string())
                });
            
            
            let token = match token_header {
                Some(t) => t.to_string(),
                None => return Ok(OptionalUser(None))
                
            };
            
            match verify_jwt(token.to_string(), state).await{
                Ok(token_data) => Ok(OptionalUser(Some(token_data.claims))),
                Err(_) => Ok(OptionalUser(None))
            }
        })
    }
}


pub struct AuthenticatedUser {
    pub user_id: i64,
    pub email: String,
}

impl FromRequest for AuthenticatedUser {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let email = request
            .extensions_mut()
            .get::<TokenData<Claims>>()
            .map(|t| AuthenticatedUser { user_id:t.claims.sub.clone(), email: t.claims.email.clone() })
            .ok_or(ApiError::Unauthorized);
        ready(email)
    }    
}

#[derive(Debug, Clone)]
pub enum Identity {
    User(Claims),
    AnonUser(String)
}


// Utopia Error Schema

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    pub success: bool,
    pub message: String
}

#[derive(IntoResponses)]
#[allow(dead_code)]
pub enum CommonErrors {
    #[response(status=401)]
    Unauthorized(ErrorBody),

    #[response(status=404)]
    NotFound(ErrorBody),

    #[response(status=500)]
    SeverError(ErrorBody)
}


struct SecurityAddon; // adds the bearer scheme to the spec

impl Modify for SecurityAddon { // hook called when the spec is built
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) { // mutable access to the spec
        let components = openapi.components.get_or_insert_with(Default::default); // ensure components exist
        components.add_security_scheme( // register the scheme under the name used in handlers
            "bearer_auth", // must match security(("bearer_auth" = []))
            SecurityScheme::Http( // HTTP auth type
                HttpBuilder::new() // builder for the scheme
                    .scheme(HttpAuthScheme::Bearer) // Authorization: Bearer <token>
                    .bearer_format("JWT") // purely informational
                    .build(), // finish
            ),
        );
    }
}

#[derive(OpenApi)] // base document; paths are collected automatically
#[openapi(
    info(
        title = "E-commerce watcher API", 
        version = "0.1.0",
        description = "Product catalog API with authentication, bookmarks, history and currency conversion." 
    ), 
    components(schemas(ErrorBody, SortValues)), 
    modifiers(&SecurityAddon), // apply the security scheme
    tags( // order here = order in the UI
        (name = "Auth", description = "Register, log-in, and refresh tokens. No token needed."), // 1st: auth routes
        (name = "Account (protected)", description = "Logout, profile, and profile updates. Requires a JWT."), // 2nd: protected routes
        (name = "Bookmarks (protected)", description = "Saved, Get, and Delete bookmarked products for the signed-in user. Requires a JWT."), // 3rd: protected routes
        (name = "History", description = "Saved, Get, and Delete Search history. (Optional User). "), // 4th: public routes
        (name = "Products", description = "Browse, semantic search and score products. Public."), // 5th: public routes
        (name = "Tools", description = "Health check, scoreing system and currency conversion. Public.") // 6th: public routes
    )
)]
pub struct ApiDoc;


// Embeding python API 

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum TextPayload {
    Single(String),
    Multiple(Vec<String>)
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum EmbeddingResponseData {
    Single(Vec<f32>),
    Multiple(Vec<Vec<f32>>),
}


#[derive(Serialize, Debug)]
pub struct RequestBody {
    pub text: TextPayload
}

#[derive(Deserialize, Debug)]
pub struct EmbedderApiResponse {
    pub data: EmbeddingResponseData,
}

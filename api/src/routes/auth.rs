use actix_web::{HttpMessage, HttpRequest, HttpResponse, post, web};
use actix_web::middleware::from_fn;
use actix_governor::Governor;
use chrono::{Utc};
use jsonwebtoken::TokenData;
use sea_orm::*;
use utoipa_actix_web::{scope::scope, service_config::ServiceConfig};

use entities::users::{self, Entity as User};
use entities::tokens::{self, Entity as Tokens};
use validator::Validate;

use crate::middleware::authentication::check_auth_middleware;
use crate::schemas::response_sch::{ApiError, ApiResponse, UserSchema, RefreshResponse};
use crate::schemas::request_sch::{CreateUser, LoginUser, RefreshTokenRequest, UpdateUser};
use crate::schemas::others::{AppState, AuthenticatedUser, Claims, CommonErrors};
use crate::utils::constants::DefaultGovernorConfig;
use crate::utils::jwt::{hash_refresh_token};
use crate::utils::pwdhash::{hash_paswrd, verify_hashpaswrd};
use crate::utils::utils_funcs::{get_stored_token, revoke_rotate_token, extract_refresh_token, generate_set_tokens, bearer_or_cookie_token};

#[utoipa::path(
    responses(
        (status=200, description="Register User", body=ApiResponse<UserSchema>),
        CommonErrors
    ),
    tag="Auth"
)]
#[post("/register")]
async fn register_user(
    payload: web::Json<CreateUser>,
    state: web::Data<AppState>,
)-> Result<HttpResponse, ApiError>{

    let payload = payload.into_inner();
    let pwrd = payload.password_text.to_string();
    
    match payload.validate() {
        Ok(_) => {
            let new_user = users::ActiveModel {
                email: Set(payload.email.to_string()),
                username: Set(payload.username.to_string()),
                password_hash: Set(hash_paswrd(pwrd)),
                created_at: Set(Utc::now()),
                joined: Set(Utc::now()),
                orders_count: Set(Some(0)),
                ..Default::default()
            };
            let db = state.db.clone();
            let model = new_user.insert(&db).await?;

            let response = UserSchema {
                id: model.id,
                username: model.username.clone(),
                email: model.email,
                orders_count:model.orders_count.unwrap_or(0),
                date_joined:model.joined,
                token: None,
                refresh_token: None
            };

            Ok(ApiResponse::new(
                201, 
                format!("Created user {}", &model.username), 
                response
            ))
        }
        Err(err) => {
            Err(ApiError::ValidationError(format!("{}", err)))
        }
    }

}


#[utoipa::path(
    responses(
        (status=200, description="Log-In User", body=ApiResponse<UserSchema>),
        CommonErrors
    ),
    tag="Auth"
)]
#[post("/login")]
async fn login_user(
    state:web::Data<AppState>, 
    payload: web::Json<LoginUser>
)-> Result<HttpResponse, ApiError>{
    let payload = payload.into_inner();
    let db = state.db.clone();

    let user_ = User::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(&db)
        .await?;

    match user_ {
        Some(model) => {
            let paswrd = verify_hashpaswrd(payload.password_text, model.password_hash.clone());
            if !paswrd{
                return Err(ApiError::Unauthorized);
            }

            let (raw_refresh, token, set_cookie) = generate_set_tokens(&state, &model, model.id).await?;

            Ok(ApiResponse::new_with_cookies(
                set_cookie[0].clone(), 
                set_cookie[1].clone(), 
                200, 
                format!("Loged in user {}", model.username), 
                UserSchema {
                    id: model.id,
                    username: model.username,
                    email: model.email,
                    orders_count: model.orders_count.unwrap_or(0),
                    date_joined:model.joined,
                    token: Some(token.clone()),
                    refresh_token: Some(raw_refresh),
                }
            ))
        }
        None => Err(ApiError::Unauthorized)
    }
}


#[utoipa::path(
    responses(
        (status=200, description="Refresh User API Token", body=ApiResponse<UserSchema>),
        CommonErrors
    ),
    security(("refresh_token" = [])),
    tag="Auth"
)]
#[post("/refresh")]
async fn refresh_token(
    state:web::Data<AppState>,
    payload:Option<web::Json<RefreshTokenRequest>>,
    request:HttpRequest
)->Result<HttpResponse, ApiError>{
    let payload = payload.map(|b| b.into_inner().refresh_token).flatten();

    let refresh_token_string = match extract_refresh_token(request, payload).await
    {
        Some(r) => r,
        None => return Err(ApiError::NotFound("Refresh Token not found".to_string()))
    };

    let hashed = hash_refresh_token(refresh_token_string);
    let db: DatabaseConnection = state.db.clone();
    
    //get stored token
    let stored = get_stored_token(&hashed, &db).await?;
    
    //get user
    let user = match User::find_by_id(stored.user_id)
        .one(&db)
        .await?
    {
        Some(u) => u,
        None => return Err(ApiError::Unauthorized),
        
    };
    
    //revoke and rotate token
    let (new_acess_token, new_raw_refresh, set_cookies) = revoke_rotate_token(stored, &user, &state).await?;

    Ok(ApiResponse::new_with_cookies(
        set_cookies[0].clone(),
        set_cookies[1].clone(),
        200,
        "Token refreshed",
        RefreshResponse {
            token: new_acess_token,
            refresh_token: new_raw_refresh.to_string(),
        },
    ))
}

// LogOut function

#[utoipa::path(
    responses(
        (status=200, description="Logout and Revoke API Token", body=ApiResponse<UserSchema>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="Account (protected)"
)]
#[post("/logout")]
async fn logout_user(
    state:web::Data<AppState>,
    request:HttpRequest,
    user: AuthenticatedUser
)->Result<HttpResponse, ApiError>{
    let token = bearer_or_cookie_token(request).await?;
    let token = match Tokens::find()
        .filter(tokens::Column::Token.eq(token))
        .one(&state.db)
        .await?
        {
            Some(t) => t,
            None => return Err(ApiError::Unauthorized)
        };

    token.delete(&state.db).await?;

    Ok(ApiResponse::new(
        200, 
        format!("User {} logout sucessfully", user.email), 
        ()
    ))
}


#[utoipa::path(
    responses(
        (status=200, description="Update User Information", body=ApiResponse<UserSchema>),
        CommonErrors
    ),
    tag="Account (protected)"
)]
#[post("/update")]
async fn update_user(state:web::Data<AppState>, request:HttpRequest, payload:web::Json<UpdateUser>)->Result<HttpResponse, ApiError>{
    let payload = payload.into_inner();

    match payload.validate() {
        Ok(_) => {
            let p_email = payload.email.clone()
                .ok_or_else(|| ApiError::ValidationError("Email field is required".to_string()))?;
            let p_name = payload.username.clone()
                .ok_or_else(|| ApiError::ValidationError("Username field is required".to_string()))?;
            let mut count = 0;
            let user_email = request.extensions_mut().get::<TokenData<Claims>>()
                .map(|token_data| token_data.claims.email.clone())
                .ok_or(ApiError::Unauthorized)?;

            let user = User::find()
                .filter(users::Column::Email.eq(user_email))
                .one(&state.db)
                .await
                .map_err(|e| ApiError::DatabaseError(e))?
                .ok_or_else(|| ApiError::NotFound("User not found ...".to_string()))?;
            

            let mut mod_user:users::ActiveModel = user.clone().into();
            if user.email != p_email{ 
                mod_user.email = Set(p_email.clone());
                count += 1;
            };
            if user.username != p_name{
                mod_user.username = Set(p_name);
                count += 1;
            };
            if count > 0 {
                let rows = mod_user.update(&state.db).await?;
                // let (raw_refresh, token, _) = generate_set_tokens(&state, &rows, user.id).await?;
                let user_resp = UserSchema{
                    id:rows.id,
                    username: rows.username.clone(),
                    email: rows.email.clone(),
                    orders_count:rows.orders_count.unwrap_or(0),
                    date_joined:rows.joined,
                    token: None,
                    refresh_token: None
                };
                Ok(ApiResponse::new(
                    200, 
                    format!("Updated user {:?} information", user.username), 
                    user_resp
                ))
            }else{
                Ok(ApiResponse::new(200, "No update to user information was made", ()))
            }
        },
        Err(err) => {
            Err(ApiError::ValidationError(format!("{}", err)))
        }
    }
    
}

pub fn config_auth_routes(cfg: &mut ServiceConfig, rate_limter: DefaultGovernorConfig) {
    cfg.service(
        scope("/auth")
            .wrap(Governor::new(&rate_limter))
            .service(register_user)
            .service(login_user)
            .service(refresh_token)
            .service(
                scope("/user")
                .wrap(from_fn(check_auth_middleware))
                .service(update_user)
                .service(logout_user)
            )
    );
}


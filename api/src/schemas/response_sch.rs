use actix_web::{HttpResponse, ResponseError, cookie::Cookie, http::StatusCode};
use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use sea_orm::FromQueryResult;
use thiserror::Error;
use serde_json::Value;
use chrono::{Utc, DateTime};
use jsonwebtoken::errors;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct UserSchema{
    pub id: i64,
    pub username: String,
    pub email: String,
    pub orders_count: i64,
    pub date_joined:DateTime<Utc>,
    pub token: Option<String>,
    pub refresh_token: Option<String>
}


#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct BookmarkSchema{
    pub total: i64,
    pub products:Vec<ProductsSchema>
}

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct HistorySchema{
    pub total: i64,
    pub search: Vec<Value>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshResponse {
    pub token: String,
    pub refresh_token: String
}

#[derive(Serialize, ToSchema)]
pub struct ApiResponse<T: Serialize> {
    pub status_code: u16,
    pub message: String,
    pub body: T,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Internal database error")]
    DatabaseError(#[from] sea_orm::DbErr),
    
    #[error("JSON processing error {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Token error {0}")]
    JwtErr(#[from] jsonwebtoken::errors::Error),

    #[error("Invalid credentials")]
    Unauthorized,

    #[error("Not found {0}")]
    NotFound(String),

    #[error("Internal server error")]
    SeverError,

    #[error("Service unavailable {0}")]
    ServiceUnavailable(String),

    #[error("Validation Failed: {0}")]
    ValidationError(String)
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(status_code: u16, message: impl Into<String>, body: T) -> HttpResponse {
        HttpResponse::build(StatusCode::from_u16(status_code).unwrap()).json(ApiResponse {
            status_code,
            message: message.into(),
            body,
        })
    }

    // for routes with jwt tokens
    pub fn new_with_cookies(
        a_t: Cookie<'static>,
        r_t: Cookie<'static>,
        status_code: u16,
        message: impl Into<String>,
        body: T,
    ) -> HttpResponse {
        HttpResponse::build(StatusCode::from_u16(status_code).unwrap())
            .cookie(a_t)
            .cookie(r_t)
            .json(ApiResponse {
                status_code,
                message: message.into(),
                body,
            })
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let (status, message) = match self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()),
            ApiError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error (DB Error)".to_string()),
            ApiError::JsonError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON payload".to_string()),
            ApiError::JwtErr(e) => match e.kind(){
                errors::ErrorKind::ExpiredSignature => {
                    (StatusCode::UNAUTHORIZED, "Token Expired".to_string())
                },
                _ => (StatusCode::UNAUTHORIZED, "Invalid Token".to_string())
            }
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::SeverError => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error: {Server Error}".to_string()),
            ApiError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.to_string()),
            ApiError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone())
        };
        ApiResponse::new(status.as_u16(), message, ())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, FromQueryResult, ToSchema)]
pub struct ProductsSchema{
    pub id: i64,
    pub product_url: String,
    pub product_name: String,
    pub product_price: Decimal,
    pub product_price_text: String,
    pub product_currency: String,
    pub product_brand_name: Option<String>,
    pub product_category: Option<String>,
    pub product_image:Option<String>,
    pub product_sku: Option<String>,
    pub product_details: Value, // this is json object
    pub product_reviews: Value, // this is json object
    pub product_specifications: Value,
    pub score: i64,
    pub created_at: DateTime<Utc>
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProductResponse{
    pub total_items: u64,
    pub total_pages: u64,
    pub current_page: u64,
    pub next_page: Option<u64>,
    pub products: Vec<ProductsSchema>
}


#[derive(FromQueryResult, Serialize, Deserialize, ToSchema)] // raw-SQL result row, also sent as JSON
pub struct ProductHit {
    #[sea_orm(nested)]
    pub products: ProductsSchema,
    pub distance: f64,
}


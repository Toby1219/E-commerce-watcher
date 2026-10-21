use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, errors::{Error, ErrorKind}, TokenData};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use chrono::{Duration, Utc};
use rand::{RngExt, distr::Alphanumeric};
use sha2::{Digest, Sha256};
use hex;
use actix_web::{web};

use crate::schemas::others::{AppState, Claims};
use crate::schemas::response_sch::ApiError;
use entities::{tokens::{self, Entity as Tokens}, users::Model};




pub fn issue_jwt(user:&Model, state:&web::Data<AppState>)->Result<String, Error>{
    let now = Utc::now();
    let expire = Duration::minutes(20);

    let claims = Claims{
        sub: user.id,
        iat: now.timestamp() as usize,
        exp: (now+expire).timestamp() as usize,
        email: user.email.to_string()
    };
    let secrets = state.env_config.secret_key.clone();

    encode(&Header::default(), &claims, &EncodingKey::from_secret(secrets.as_ref()))
}

pub async  fn verify_jwt(jwt: String, state:web::Data<AppState>)->Result<TokenData<Claims>, ApiError>{
    let secrets = state.env_config.secret_key.clone();
    let claims_data: TokenData<Claims> = decode(
        jwt.clone(), 
        &DecodingKey::from_secret(secrets.as_bytes()), 
        &Validation::default())?;
    
    let token = match Tokens::find()
        .filter(tokens::Column::Token.eq(&jwt))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?
        {
            Some(t) => t,
            None => return Err(ApiError::NotFound("User token not found".to_string()))
        };
    
    if token.revoked == true{
        return Err(ApiError::JwtErr(Error::from(ErrorKind::InvalidToken)));
    }
    
    Ok(claims_data)
}

pub fn generate_refresh_token() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

pub fn hash_refresh_token(token: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(&token.as_bytes());
    hex::encode(&hasher.finalize())
}
 

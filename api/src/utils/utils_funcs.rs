use sea_orm::*;
use chrono::{Utc, Duration};
use actix_web::{HttpRequest, web};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use uuid::Uuid;
use actix_web::cookie::{Cookie, SameSite, time::Duration as CookieDuration};
use reqwest::Client;

use entities::tokens::{self, Entity as Tokens};
use entities::users::{self, Entity as User};
use entities::products::{self, Entity as Products};
use entities::bookmark::{self, Entity as Bookmark};
use entities::history::{self, Entity as History};

use crate::schemas::others::{AppState, OptionalUser};
use crate::schemas::response_sch::{ApiError, BookmarkSchema, HistorySchema, ProductsSchema};
use crate::utils::constants::set_cookies;
use crate::utils::jwt::{issue_jwt, generate_refresh_token, hash_refresh_token};
use crate::schemas::request_sch::{BHOQuery};
use crate::schemas::others::Identity;
use crate::schemas::others::{TextPayload, RequestBody};


pub async fn bearer_or_cookie_token(req:HttpRequest)-> Result<String, ApiError>{
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
    match token_header {
        Some(token) => Ok(token),
        None => Err(ApiError::Unauthorized)
    }
}

pub async fn get_user_identity(user: OptionalUser, req:&HttpRequest)->(Identity, Option<Cookie<'static>>){
    let identity = match user.0 {
        Some(claims) => {
            (Identity::User(claims), None)
        },
        None => {
            let (anon_id, new_cookie) = get_create_anon_id(&req).await;
            (Identity::AnonUser(anon_id), new_cookie)
        }
    };

    identity
}

pub async fn get_create_anon_id(req:&HttpRequest)->(String, Option<Cookie<'static>>){
    if let Some(cookie) = req.cookie("anon_id"){
        (cookie.value().to_string(), None)
    }
    else {
        let new_id = Uuid::new_v4().to_string();
        let cookie = Cookie::build("anon_id", new_id.clone())
            .path("/")
            .http_only(true)
            .same_site(SameSite::None)
            .secure(true)
            .max_age(CookieDuration::days(1))
            .finish();
        (new_id, Some(cookie))
    }
}

pub async fn extract_user_ip(request:&HttpRequest)->String{
    let direct_ip = request.peer_addr().map(|addr| addr.ip().to_string());
    let real_ip_addr = request.connection_info().realip_remote_addr().map(|addr| addr.to_string());
    real_ip_addr.or(direct_ip).unwrap_or_else(|| "Unknow".to_string())
}

pub async fn get_stored_token(hashed:&str, db:&DatabaseConnection)->Result<tokens::Model, ApiError>{
    let stored = Tokens::find()
        .filter(tokens::Column::HashToken.eq(hashed)) 
        .order_by_desc(tokens::Column::CreatedAt)
        .one(db)
        .await
        .map_err(ApiError::DatabaseError)?
        .ok_or(ApiError::Unauthorized)?;

    if stored.revoked == true || stored.expired_at < Utc::now() {
        return Err(ApiError::Unauthorized);
    }
    Ok(stored)
}

pub async fn generate_set_tokens(state:&web::Data<AppState>, user:&users::Model, id:i64)->Result<(String, String, Vec<Cookie<'static>>), ApiError>{
    let raw_refresh = generate_refresh_token();
    let hashed = hash_refresh_token(raw_refresh.clone());

    let token = match issue_jwt(&user, state) {
        Ok(t) => t,
        Err(_) => String::new()
        
    };

    let refresh_model = tokens::ActiveModel {
        id: Default::default(),
        user_id: Set(id),
        token: Set(token.clone()),
        hash_token: Set(hashed.clone()),
        expired_at: Set(Utc::now() + Duration::days(1)),
        revoked: Set(false),
        created_at: Set(Utc::now()),
        ..Default::default()
    };

    refresh_model.insert(&state.db).await?;

    let set_cookie: Vec<Cookie<'_>> = set_cookies(token.to_string(), raw_refresh.to_string());
    Ok((raw_refresh, token, set_cookie))

}

pub async fn revoke_rotate_token(
    stored:tokens::Model, 
    user:&users::Model,
    state:&web::Data<AppState>
)->Result<(String, String, Vec<Cookie<'static>>), ApiError>{
    let user_id = stored.user_id;
    let mut active: tokens::ActiveModel = stored.into();
    
    active.revoked = Set(true);
    active.update(&state.db).await?;

    let (new_acess_token, new_raw_refresh, set_cookie) = generate_set_tokens(state, user, user_id).await?;
    
    Ok((new_acess_token, new_raw_refresh, set_cookie))
}

pub async fn extract_refresh_token(request:HttpRequest, jbody:Option<String>)-> Option<String> {
    if let Some(cookie) = request.cookie("refresh_token"){
        return Some(cookie.value().to_string())
    }

    if let Some(header_val) = request.headers().get("X-Refresh-Token"){
        if let Ok(token_str) = header_val.to_str(){
            return Some(token_str.to_string());   
        }
    }

    if let Some(b) = jbody{
        return Some(b.clone());
    }

    None
}

pub async fn extract_user(db:&DatabaseConnection, email: String) -> Result<users::Model, ApiError>{
    let current_user = email;

    let user = User::find()
        .filter(users::Column::Email.eq(&current_user))
        .one(db)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?
        .ok_or_else(|| ApiError::NotFound("No user with that email".to_string()))?;
        
    Ok(user)
}

pub async fn extract_product(payload:&BHOQuery, db:&DatabaseConnection) ->Result<products::Model, ApiError> {
    let product = Products::find()
        .filter(products::Column::Id.eq(payload.product_id))
        .one(db)
        .await.map_err(|e| ApiError::DatabaseError(e))?
        .ok_or_else(|| ApiError::NotFound("No Product is found".to_string()))?;  
    Ok(product)
}

pub async  fn extract_bookmarks(user_id:i64, db:&DatabaseConnection)->Result<BookmarkSchema, ApiError>{
    let book_marks = Bookmark::find()
        .filter(bookmark::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?;

    let prod_ids:Vec<i64> = book_marks.into_iter().map(|m| m.product_id).collect();

    let products = Products::find()
        .filter(products::Column::Id.is_in(prod_ids))
        .all(db)
        .await?;

    Ok(BookmarkSchema { 
        total: products.len() as i64, 
        products: products.into_iter().map(
            |p| ProductsSchema{
                id: p.id,
                product_url: p.product_url,
                product_name: p.product_name,
                product_brand_name: p.product_brand_name,
                product_sku: p.product_sku,
                product_price: p.product_price,
                product_price_text: p.product_price_text,
                product_currency: p.product_currency,
                product_image: p.product_image,
                product_category: p.product_category,
                product_details: p.product_details,
                product_reviews: p.product_reviews,
                product_specifications: p.product_specifications,
                created_at:p.created_at,
                score:p.score
            }).collect()
    })
}

pub async fn extract_history(user_id:Option<i64>, anon_user:Option<String>, db:&DatabaseConnection)->Result<HistorySchema, ApiError> {
    let stmt = match (user_id, anon_user) {
        (Some(u), _) => history::Column::UserId.eq(u),

        (None, Some(anon)) => history::Column::AnonUser.eq(anon),

        (None, None) => return Ok(HistorySchema { total: 0, search: Vec::new() }),
    };
    let history_model = History::find()
        .filter(stmt)
        .all(db)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?;

    let h_values:Vec<JsonValue> = history_model.into_iter().map(|h| json!({"phrase":&h.phrase, "created_at":&h.created_at})).collect();

    Ok(HistorySchema { 
        total: h_values.len() as i64, 
        search:h_values
    })  
    
}

pub async fn extract_tokens_db(db:&DatabaseConnection, user_id:i64)->Result<String, ApiError> {
    let tokens = Tokens::find()
        .filter(tokens::Column::UserId.eq(user_id))
        .order_by_desc(tokens::Column::CreatedAt)
        .one(db)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?
        .ok_or_else(|| ApiError::NotFound("User token not found".to_string()))?;

    let acess_token = tokens.token;

    Ok(acess_token)    
}

pub async fn save_user_history(db:&DatabaseConnection, phrase: String, identity:&Identity) -> Result<Value, ApiError>{
    match &identity {
        Identity::User(user) => {
            let old_history = extract_history(Some(user.sub), None, db).await?;
            let existing = old_history.search.iter().any(|p| {
                p.get("phrase")
                    .and_then(|value| value.as_str())
                    .map(|stored_phrase| stored_phrase == phrase)
                    .unwrap_or(false)
            });

            if existing {
                return Ok(json!({"total":&old_history.total}));
            }

            let history = history::ActiveModel {
                id: Default::default(),
                phrase: Set(phrase),
                user_id: Set(Some(user.sub)),
                anon_user: Set(None),
                created_at: Set(Utc::now()),
                ..Default::default()
            };
            let _ = history.insert(db).await?;
            let history = extract_history(Some(user.sub), None, db).await?;

            Ok(json!({"total":&history.total}))
        }
        Identity::AnonUser(annon_id) => {
            let old_history = extract_history(None, Some(annon_id.to_string()), db).await?;
            let existing = old_history.search.iter().any(|p| {
                p.get("phrase")
                    .and_then(|value| value.as_str())
                    .map(|stored_phrase| stored_phrase == phrase)
                    .unwrap_or(false)
            });

            if existing {
                return Ok(json!({"total":&old_history.total}));
            }

            let history = history::ActiveModel {
                id: Default::default(),
                phrase: Set(phrase),
                user_id: Set(None),
                anon_user: Set(Some(annon_id.to_string())),
                created_at: Set(Utc::now()),
                ..Default::default()
            };
            let _ = history.insert(db).await?;
            let history = extract_history(None, Some(annon_id.clone()), db).await?;

            Ok(json!({"total":&history.total}))
        }
    }
}

pub async fn make_request<T: DeserializeOwned + std::fmt::Debug>(
    query:Option<TextPayload>, 
    url_path: String, 
    host: String,
    method: String
)->Result<T, ApiError> {
    let client = Client::new();
    let url = format!("{}/{}", host, url_path);

    let request_body = match query {
     Some(q) => RequestBody { text: q},
     None => RequestBody {text: TextPayload::Single("".to_string())}  
    };

    let response = match method.as_str() {
        "get" => {
            client
                .get(url)
                .send()
                .await
                .map_err(|e| ApiError::NotFound(format!("[External API request failed] ERROR: {}", e)))?  
        },
        "post" => {
            client
                .post(url)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| ApiError::ServiceUnavailable(format!("[External API request failed] ERROR: {}", e)))?  
        },
        _ => return Err(ApiError::NotFound(format!("Unsupported HTTP method: {}", method))),
    };
    if !response.status().is_success() {
        return Err(ApiError::ServiceUnavailable(format!(
            "[External API returned non-success status] STATUS: {}",
            response.status()
        )));
    }
  
    let api_response: T = response
        .json()
        .await
        .map_err(|e| ApiError::ServiceUnavailable(format!("[failed to parse API response json] ERROR: {}", e)))?;
    log::info!("response {:?}", api_response);
    Ok(api_response)

}





use std::str::FromStr;
use actix_web::middleware::from_fn;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, get, post, web};
use sea_orm::{Statement, DbBackend, prelude::PgVector};
use sea_orm::{ActiveModelTrait, ColumnTrait, Condition, EntityTrait, FromQueryResult, ModelTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};
use serde_json::{json, Value};
use actix_governor::{Governor};
use chrono::{Utc};
use jsonwebtoken::TokenData;
use utoipa_actix_web::{scope::scope, service_config::ServiceConfig};

use crate::middleware::authentication::check_auth_middleware;
use crate::utils::constants::DefaultGovernorConfig;
use crate::schemas::response_sch::{
    ApiError, ApiResponse, ProductResponse, 
    ProductsSchema, UserSchema, ProductHit, HistorySchema,
    BookmarkSchema
};
use crate::schemas::others::{
    AppState, AuthenticatedUser, Claims, 
    OptionalUser, Identity, CommonErrors,
    TextPayload,
};
use crate::schemas::request_sch::{
    BHOQuery, Converstions, CurrencyQuery, 
    ProductIdQuery, ProductQuery, SortValues, SearchReq
};
use crate::middleware::redis_cache::CacheReqResp;
use crate::utils::utils_funcs::{
    extract_bookmarks, extract_history, extract_product, 
    extract_tokens_db, extract_user, extract_user_ip,
    get_user_identity, save_user_history, make_request,
};
use crate::utils::text_embedder::embed_one;

use entities::products::{self, Entity as Products};
use entities::bookmark::{self, Entity as Bookmark};
use entities::history::{self, Entity as History};
use entities::users::{self, Entity as User};
use entities::currencies::{self, Entity as Currency};


#[utoipa::path(
    responses(
        (status=200, description="Api Health", body=ApiResponse<Value>),
        CommonErrors
    ),
    tag="Tools"
)]
#[get("/health")]
async fn health_check(state:web::Data<AppState>, request:HttpRequest)-> Result<HttpResponse, ApiError>{
    let id_ = extract_user_ip(&request).await;
    let key = CacheReqResp::key_for("health", &id_);
    if let Some(cached_data) = state.redis.get::<Value>(&key).await {
        return Ok(ApiResponse::new(
            200, 
            "Checking API health (From Cached Data)".to_string(),
            cached_data
        ));
    }
    
    let current_user = request.extensions_mut().get::<TokenData<Claims>>()
        .map(|token_data| token_data.claims.email.clone())
        .unwrap_or_else(|| "anonymous user".to_string());

    let body = json!({"message": "API says am good to go ...", "current_user":current_user});

    state.redis.set(&key, &body, 60).await.ok();

    Ok(ApiResponse::new(
        200, 
        "Checking API health".to_string(), 
        body
    ))
}

#[utoipa::path(
    responses(
        (status=200, description="User Information", body=ApiResponse<UserSchema>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="Account (protected)"
)]
#[get("/me")]
async fn get_user(state:web::Data<AppState>, user: AuthenticatedUser)->Result<HttpResponse, ApiError> {
    // let id_ = extract_user_ip(&request).await;
    let email = user.email;
    // let cached_key = CacheReqResp::key_for(format!("{}:user", &email).as_ref(), &id_);
    // if let Some(cached_data) = state.redis.get::<UserSchema>(&cached_key).await {
    //     return Ok(ApiResponse::new(200, "User data from cache".to_string(), cached_data));
    // }
    let user = User::find()
        .filter(users::Column::Email.eq(&email))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?
        .ok_or_else(|| ApiError::NotFound(format!("User with email ({}) not found", email)))?;
    
    let a = extract_tokens_db(&state.db, user.id).await?;
    let user_resp = UserSchema{
        id:user.id,
        username: user.username,
        email: user.email,
        orders_count:user.orders_count.unwrap_or(0),
        date_joined:user.joined,
        token: Some(a),
        refresh_token: None
    };
    // let _ = state.redis.set(&cached_key, &user_resp, 120).await;
    
    Ok(ApiResponse::new(
        200, 
        format!("Current user information"), 
        user_resp
    ))
}

// Search Product home page and search
#[utoipa::path(
    params(ProductQuery),
    responses(
        (status=200, description="Get Multiple Products", body=ApiResponse<ProductResponse>),
        CommonErrors
    ),
    tag="Products"
)]
#[get("/products")]
async fn multiple_product(state:web::Data<AppState>, query:web::Query<ProductQuery>, request:HttpRequest)->Result<HttpResponse, ApiError> {
    let query = query.into_inner();
    let page = query.p.unwrap_or(1).max(1);
    let per_page = query.p_p.unwrap_or(10).clamp(1, 100);
    let id_ = extract_user_ip(&request).await;
    let sort_type = query.s_t.unwrap_or(SortValues::ID).to_column();
    let prod_name = query.p_n.clone().unwrap_or("all products".to_string());

    let p_n = match &query.p_n {
        Some(p_n) if !p_n.trim().is_empty() => {
            p_n.to_string()
        },
        _ => return Err(ApiError::NotFound("Invalid product".to_string())),
    };

    let escaped = p_n.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let select = Products::find()
        .filter(products::Column::ProductName.ilike(format!("%{escaped}%")))
        .order_by_asc(sort_type);

    let cached_key = CacheReqResp::key_for(format!("{} <-> {} <-> {}", prod_name, page, per_page).as_ref(), &id_);
    if let Some(cached_data) = state.redis.get::<ProductResponse>(&cached_key).await {
        return Ok(ApiResponse::new(200, format!("Products from cached data"), cached_data));
    }

    let products_pag = select
    .paginate(&state.db, per_page);

    let total_items= products_pag.num_items().await?;
    let total_pages = products_pag.num_pages().await?;
    let items = products_pag.fetch_page(page - 1).await?;
    let next_page = if page < total_pages { Some(page + 1) } else { None };

    if total_items == 0 {
        let q = TextPayload::Single(p_n.clone());
        actix_web::rt::spawn(make_request(q, "run-scrapy".to_string()));
    }

    let products_result: ProductResponse = ProductResponse {
        total_items,
        total_pages,
        current_page:page,
        next_page,
        products: items.into_iter().map(|v| ProductsSchema{
            id: v.id,
            product_url: v.product_url,
            product_sku: v.product_sku,
            product_name: v.product_name,
            product_brand_name: v.product_brand_name,
            product_category: v.product_category,
            product_details: v.product_details,
            product_image: v.product_image,
            product_price: v.product_price,
            product_reviews: v.product_reviews,
            product_specifications: v.product_specifications,
            created_at: v.created_at,
            score: v.score
        })   
        .collect()
    };
    if total_items > 0 {
        let _ = state.redis.set(&cached_key, &products_result, 120).await;
    }
    let message = if total_items == 0 {
        format!("No products for {} yet. We're fetching them, check back in a minute.", p_n)
    } else {
        format!("Fetched products for {}", p_n)
    };

    Ok(ApiResponse::new(
        200, 
        message, 
        products_result,
    ))
}

#[utoipa::path(
    params(ProductIdQuery),
    responses(
        (status=200, description="Get Signle Product", body=ApiResponse<ProductResponse>),
        CommonErrors
    ),
    tag="Products"
)]
#[get("/product")]
async fn single_product(state:web::Data<AppState>, query:web::Query<ProductIdQuery>, request:HttpRequest)->Result<HttpResponse, ApiError>{
    let query = query.into_inner();
    let id_ = extract_user_ip(&request).await;

    let cached_key = CacheReqResp::key_for(&query.n.to_string(), &id_);
    if let Some(cached_data) = state.redis.get::<ProductsSchema>(&cached_key).await {
        return Ok(ApiResponse::new(200, format!("Product from cached data"), cached_data));
    }

    let product = match Products::find()
        .filter(products::Column::Id.eq(query.n))
        .one(&state.db)
        .await.map_err(|e| ApiError::DatabaseError(e))?
        {
            Some(t) => t,
            None => return Err(ApiError::NotFound("Product not found".to_string()))
        };

    let product_result = ProductsSchema{
        id: product.id,
        product_url: product.product_url,
        product_name: product.product_name.clone(),
        product_brand_name: product.product_brand_name,
        product_image: product.product_image,
        product_price: product.product_price,
        product_sku: product.product_sku,
        product_category: product.product_category,
        product_specifications: product.product_specifications,
        product_details: product.product_details,
        product_reviews: product.product_reviews,
        created_at: product.created_at,
        score: product.score,
    };
    let _ = state.redis.set(&cached_key, &product_result, 120).await;
    Ok(ApiResponse::new(
        200, 
        format!("{}", product.product_name), 
        product_result,
    ))
}

#[utoipa::path(
    responses(
        (status=200, description="Get User Bookmarks", body=ApiResponse<BookmarkSchema>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="Bookmarks (protected)"
)]
#[get("/bookmark")]
async fn get_bookmark_handler(state:web::Data<AppState>, user: AuthenticatedUser)->Result<HttpResponse, ApiError>{
    // let id_ = extract_user_ip(&request).await;
    // let cached_key = CacheReqResp::key_for(format!("{}:book_mark",&current_user_email).as_ref(), &id_);
    // if let Some(cached_data) = state.redis.get::<BookmarkSchema>(&cached_key).await {
    //     return Ok(ApiResponse::new(200, "Bookmark data from cache".to_string(), cached_data));
    // }
    let bookmarks = extract_bookmarks(user.user_id, &state.db).await?;
    // let _ = state.redis.set(&cached_key, &bookmarks, 120).await;
    Ok(ApiResponse::new(200, "User Bookmark".to_string(), bookmarks))
}

#[utoipa::path(
    params(BHOQuery),
    responses(
        (status=200, description="Save User Bookmark", body=ApiResponse<BookmarkSchema>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="Bookmarks (protected)"
)]
#[post("/bookmark")]
async fn bookmark_handler(state:web::Data<AppState>, payload:web::Json<BHOQuery>, user_auth:AuthenticatedUser)->Result<HttpResponse, ApiError>{
    let payload = payload.into_inner();
    let user = extract_user(&state.db, user_auth.email).await?;
    let product = extract_product(&payload, &state.db).await?;
    
    let book_mark = bookmark::ActiveModel {
        id: Default::default(),
        product_id: Set(product.id),
        user_id: Set(user.id),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    book_mark.insert(&state.db).await?;
    let bookmarks = extract_bookmarks(user.id, &state.db).await?;

    Ok(ApiResponse::new(
        201, 
        format!("Created Bookmark for user {}", user.username),
        json!({
            "total":&bookmarks.total,
            "product":ProductsSchema{
                id: product.id,
                product_url: product.product_url,
                product_name: product.product_name,
                product_brand_name: product.product_brand_name,
                product_sku: product.product_sku,
                product_price: product.product_price,
                product_image: product.product_image,
                product_category: product.product_category,
                product_details: product.product_details,
                product_reviews: product.product_reviews,
                product_specifications: product.product_specifications,
                created_at:product.created_at,
                score:product.score,
            }
        })
    ))
}

#[utoipa::path(
    params(BHOQuery),
    responses(
        (status=200, description="Delete User Bookmark", body=ApiResponse<BookmarkSchema>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="Bookmarks (protected)"
)]
#[post("/del/bookmark")]
async fn del_bookmark_handler(state:web::Data<AppState>, payload:web::Json<BHOQuery>, user_auth:AuthenticatedUser)->Result<HttpResponse, ApiError>{
    let payload = payload.into_inner();
    let user = extract_user(&state.db, user_auth.email).await?;
    let condition = Condition::all()
        .add(bookmark::Column::UserId.eq(user.id))
        .add(bookmark::Column::ProductId.eq(payload.product_id));

    let book_mark = match Bookmark::find()
        .filter(condition)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?
        {
            Some(b) => b,
            None => return  Err(ApiError::NotFound("Book mark data not found".to_string()))
        };
    let book_id = book_mark.id;
    
    let del_book = book_mark.delete(&state.db).await?;
    let bookmarks = extract_bookmarks(user.id, &state.db).await?;

    Ok(ApiResponse::new(
        200, 
        format!("Delete bookmark {}", del_book.rows_affected),
        json!({
            "total":&bookmarks.total,
            "removedId":book_id
        })
    ))

}

#[utoipa::path(
    responses(
        (status=200, description="Get User Histories", body=ApiResponse<HistorySchema>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="History"
)]
#[get("/history")]
async fn get_history_handler(state:web::Data<AppState>, user_auth: OptionalUser, request:HttpRequest)->Result<HttpResponse, ApiError>{
    let (identity, new_cookie) = get_user_identity(user_auth, &request).await;

    match identity {
        Identity::User(user) => {
            let id_ = extract_user_ip(&request).await;
            let cached_key = CacheReqResp::key_for(format!("{}:history", &user.email).as_ref(), &id_);
                    
            if let Some(cached_data) = state.redis.get::<HistorySchema>(&cached_key).await {
                return Ok(ApiResponse::new(200, "History data from cache".to_string(), cached_data));
            }

            let history = extract_history(Some(user.sub), None, &state.db).await?;
            let _ = state.redis.set(&cached_key, &history, 120).await;

            Ok(ApiResponse::new(200, "User history".to_string(), history))
        },
        Identity::AnonUser(annon_user) => {
            let id_ = extract_user_ip(&request).await;
            let cached_key = CacheReqResp::key_for(format!("{}:history", &annon_user).as_ref(), &id_);
                    
            if let Some(cached_data) = state.redis.get::<HistorySchema>(&cached_key).await {
                return Ok(ApiResponse::new(200, "History data from cache".to_string(), cached_data));
            }
            
            let history = extract_history(None, Some(annon_user), &state.db).await?;
            let _ = state.redis.set(&cached_key, &history, 2).await;

            let mut response = ApiResponse::new(200, "User history".to_string(), history);
            if let Some(c) = new_cookie {
                let _ = response.add_cookie(&c);
            };
            Ok(response)
        }
    }
}

#[utoipa::path(
    params(BHOQuery),
    responses(
        (status=200, description="Save User History", body=ApiResponse<HistorySchema>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="History"
)]
#[post("/history")]
async fn history_handler(state:web::Data<AppState>, payload:web::Json<BHOQuery>, user_auth:OptionalUser, req:HttpRequest)->Result<HttpResponse, ApiError>{
    let (identity, new_cookie) = get_user_identity(user_auth, &req).await;
    let payload = payload.into_inner();

    let phrase_wrd = payload.phrase.expect("No phrase is sent");
    let u_h = save_user_history(&state.db, phrase_wrd, &identity).await?;

    match identity {
        Identity::User(user_claim) => {
            let user = extract_user(&state.db, user_claim.email).await?;

            Ok(ApiResponse::new(
                201, 
                format!("Created History for user {}", user.username),
                u_h
            ))
        },
        Identity::AnonUser(annon_user) => {
            let mut response = ApiResponse::new(
                201, 
                format!("Created History for user {}", annon_user),
                u_h
            );
            if let Some(c) = new_cookie {
                let _ = response.add_cookie(&c);
            };

            Ok(response)
        }
    }


}

#[utoipa::path(
    params(BHOQuery),
    responses(
        (status=200, description="Delete User History", body=ApiResponse<HistorySchema>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="History"
)]
#[post("/del/history")]
async fn del_history_handler(state:web::Data<AppState>, payload:web::Json<BHOQuery>, user_auth:OptionalUser, req:HttpRequest)->Result<HttpResponse, ApiError>{
    let (identity, new_cookie) = get_user_identity(user_auth, &req).await;
    let payload = payload.into_inner();
    
    match identity {
        Identity::User(user) => {
            let condition = Condition::all()
                .add(history::Column::UserId.eq(user.sub))
                .add(history::Column::Phrase.eq(payload.phrase.unwrap_or_else(|| "".to_string())));

            let history = match History::find()
                .filter(condition)
                .one(&state.db)
                .await
                .map_err(|e| ApiError::DatabaseError(e))?
                {
                    Some(b) => b,
                    None => return  Err(ApiError::NotFound("History data not found".to_string()))
                };
            
            let history_id = history.id;
            let history_phrase = history.phrase.clone();
            let del_history = history.delete(&state.db).await?;
            let history_db = extract_history(Some(user.sub), None,  &state.db).await?;

            Ok(ApiResponse::new(
                200, 
                format!("Delete history {}", del_history.rows_affected),
                json!({
                    "total":&history_db.total,
                    "removedId":history_id,
                    "phrase":history_phrase
                })
            ))
        },
        Identity::AnonUser(annon_id) => {
            let condition = Condition::all()
                .add(history::Column::AnonUser.eq(annon_id.clone()))
                .add(history::Column::Phrase.eq(payload.phrase.unwrap_or_else(|| "".to_string()))); 
                                
            let history = match History::find()
                .filter(condition)
                .one(&state.db)
                .await
                .map_err(|e| ApiError::DatabaseError(e))?
                {
                    Some(b) => b,
                    None => return  Err(ApiError::NotFound("History data not found".to_string()))
                };
            
            let history_id = history.id;
            let history_phrase = history.phrase.clone();
            let del_history = history.delete(&state.db).await?;
            let history_db = extract_history(None, Some(annon_id), &state.db).await?;
            
            let mut response = ApiResponse::new(
                200, 
                format!("Delete history {}", del_history.rows_affected),
                json!({
                    "total":&history_db.total,
                    "removedId":history_id,
                    "phrase":history_phrase
                })
            );
            if let Some(c) = new_cookie {
                let _ = response.add_cookie(&c);
            };
            Ok(response)
        }
    }

}

#[utoipa::path(
    params(CurrencyQuery),
    responses(
        (status=200, description="Currency Conversion", body=ApiResponse<Value>),
        CommonErrors
    ),
    tag="Tools"
)]
#[get("/currency-converstion")]
async fn currencies_converstion(state:web::Data<AppState>, payload:web::Query<CurrencyQuery>, request:HttpRequest)->Result<HttpResponse, ApiError>{
    let payload = payload.into_inner();
    let conversion = Converstions::from_str(&payload.converstion_type)?;
    let id_ = extract_user_ip(&request).await;
    let to_value = match &payload.to{
        Some(v) => v.to_string(),
        None => format!("USD")
    };

    let cached_key = CacheReqResp::key_for(format!("{} - {} - {} - {}", payload.ammount, payload.converstion_type, payload.from, to_value).as_ref(), &id_);
    if let Some(cached_data) = state.redis.get::<Value>(&cached_key).await {
        return Ok(ApiResponse::new(200, format!("Currency convention from cached data"), cached_data));
    }

    let cur_one = Currency::find()
        .filter(currencies::Column::Currency.eq(&payload.from))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?
        .ok_or_else(|| ApiError::NotFound("Currency not found for rate one".to_string()))?;
    let rate_one = cur_one.rates;

    let rate_two = match conversion {
        Converstions::OthersToOthers => {
            let code_two = payload.to.as_ref()
                .ok_or_else(|| ApiError::NotFound("currency_two is required for OTO".to_string()))?;

            let cur_two = Currency::find()
                .filter(currencies::Column::Currency.eq(code_two))
                .one(&state.db)
                .await
                .map_err(|e| ApiError::DatabaseError(e))?
                .ok_or_else(|| ApiError::NotFound("Currency not found for rate two".to_string()))?;

            Some(cur_two.rates)     
        }
        _ => None,  
    };

    let result = conversion.convert(payload.ammount, rate_one, rate_two)?;

    let results_json = serde_json::json!({
        "amount": payload.ammount,
        "from": &payload.from,
        "to": to_value,
        "converted": result,
    });

    if let Err(e) = state.redis.set(&cached_key, &results_json, 120).await {
        tracing::warn!("failed to cache currency conversion: {e}");
    }
   Ok(ApiResponse::new(
        200, 
        format!("Curency converstion"), 
        results_json
    ))
}

// routes to count orders (When user click a product link comunicate with api before redirecting to the website)
#[utoipa::path(
    params(BHOQuery),
    responses(
        (status=200, description="Product Order Count enter product id to record order counts then check user information to see changes", body=ApiResponse<Value>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="Tools"
)]
#[get("/order")]
async fn user_orders(state:web::Data<AppState>, payload:web::Query<BHOQuery>, user_auth:AuthenticatedUser)->Result<HttpResponse, ApiError> {
    let payload = payload.into_inner();
    let user = extract_user(&state.db, user_auth.email).await?;
    let products = extract_product(&payload, &state.db).await?;
    let mut mod_user:users::ActiveModel = user.into();
    let order_count = mod_user.orders_count.clone().unwrap().unwrap_or(0) + 1;
    mod_user.orders_count = Set(Some(order_count));
    mod_user.update(&state.db).await?;
    Ok(ApiResponse::new(
        200, 
        format!("user ordered {}", products.product_name), 
        json!({
            "product_url":&products.product_url
        })
    ))

}

// to calculate product scores
#[utoipa::path(
    params(BHOQuery),
    responses(
        (status=200, description="Score Product", body=ApiResponse<Value>),
        CommonErrors
    ),
    security(("bearer_auth" = [])),
    tag="Tools"
)]
#[get("/hit")]
async fn product_score(state:web::Data<AppState>, payload:web::Query<BHOQuery>)->Result<HttpResponse, ApiError>{
    let payload = payload.into_inner();
    let products = extract_product(&payload, &state.db).await?;
    let mut mod_product:products::ActiveModel = products.clone().into();
    let score = mod_product.score.clone().unwrap() + 1;
    mod_product.score = Set(score);
    mod_product.update(&state.db).await?;
    Ok(ApiResponse::new(200, format!("Scored product {}", products.product_name), ()))

}

// Searh Handler
#[utoipa::path(
    params(SearchReq),
    responses(
        (status=200, description="Search Products", body=ApiResponse<Vec<ProductHit>>),
        CommonErrors
    ),
    tag="Products"
)]
#[get("/search")]
async fn search_product(state:web::Data<AppState>, user:OptionalUser, payload: web::Query<SearchReq>, req:HttpRequest)->Result<HttpResponse, ApiError> {
    let (identity, new_cookie) = get_user_identity(user, &req).await;
    
    let ip = extract_user_ip(&req).await;
    let query = payload.query.trim().to_string();
    let limit = payload.limit.unwrap_or(5).clamp(1, 50);
    
    if query.is_empty(){
        return Err(ApiError::NotFound("Query string not found".to_string()));
    }

    let cached_key = CacheReqResp::key_for(format!("search:{:?}:query:{}:limit:{}", &identity, &query, limit).as_ref(), &ip);

    if let Some(cached_data) = state.redis.get::<Vec<ProductHit>>(&cached_key).await {
        return Ok(ApiResponse::new(200, format!("search query [ {} ] from cache", &query), cached_data));
    }

    let _ = save_user_history(&state.db, query.clone(), &identity).await?;

    let query_vector = embed_one(query.clone()).await
        .map_err(|e| ApiError::NotFound(format!("Error: {}", e)))?;

    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres, 
        "SELECT id, product_url, product_name, product_price, product_brand_name, product_category, \
            product_image, product_sku, product_details, product_reviews, product_specifications, \
            score, created_at, \
            (embedding <=> $1)::double precision AS distance \
        FROM products WHERE embedding IS NOT NULL \
        ORDER BY embedding <=> $1 LIMIT $2", 
        [PgVector::from(query_vector).into(), limit.into()]
    );

    let hits: Vec<ProductHit> = ProductHit::find_by_statement(stmt)
        .all(&state.db).await
        .map_err(ApiError::DatabaseError)?;

    let _ = state.redis.set(&cached_key, &hits, 120).await;
    let mut response = ApiResponse::new(
        200, 
        format!("searched [ {} ]", query), 
        hits
    );
    if let Some(c) = new_cookie {
        let _ = response.add_cookie(&c);
    };
    Ok(response)
}

pub fn config_views_routes(cfg: &mut ServiceConfig, ratelimiter:DefaultGovernorConfig){
    cfg.service(
        scope("/api")
        .wrap(Governor::new(&ratelimiter))
        .service(health_check)
        .service(multiple_product)
        .service(single_product)
        .service(currencies_converstion)
        .service(search_product)
        .service(history_handler)
        .service(del_history_handler)
        .service(get_history_handler)
        .service(product_score)
        .service(user_orders)
        .service(
            scope("/user")
            .wrap(from_fn(check_auth_middleware))
            .service(bookmark_handler)
            .service(del_bookmark_handler)
            .service(get_bookmark_handler)
            .service(get_user)
        )
    );
}
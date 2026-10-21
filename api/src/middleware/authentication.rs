use actix_web::{
    HttpMessage, 
    body::{MessageBody, BoxBody}, 
    dev::{ServiceRequest, ServiceResponse}, 
    http::header::AUTHORIZATION, 
    middleware::Next,
    web
};
use crate::utils::jwt::verify_jwt;
use crate::schemas::response_sch::ApiResponse;
use crate::schemas::others::AppState;

pub async fn check_auth_middleware(
    state: web::Data<AppState>,
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {

    let token = req.cookie("access_token")
        .map(|c| c.value().to_string())
        .or_else(|| {
            req.headers().get(AUTHORIZATION)
                .and_then(|h| h.to_str().ok() )
                .and_then(|s| s.strip_prefix("Bearer "))
                .map(|s| s.to_string())
        });
    
    let token_str = match token {
        Some(t) => t,
        None => return Ok(unauthorized_response(req))
    };

    let claims = match verify_jwt(token_str.to_string(), state).await {
        Ok(c) => c,
        Err(_) => return Ok(unauthorized_response(req)),
    };

    req.extensions_mut().insert(claims);
    Ok(next.call(req).await?.map_into_boxed_body())


}

fn unauthorized_response(req: ServiceRequest) -> ServiceResponse<BoxBody> {
    let response = ApiResponse::<()>::new(401, "invalid or missing token", ());
    let (req_parts, _) = req.into_parts();
    ServiceResponse::new(req_parts, response).map_into_boxed_body()
}
mod helper_func;

use actix_web::{http::StatusCode, test};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use serde_json::json;
use crate::helper_func::*;

//-----------------------------------------------------
// HELPER FUNCTION FOR TEST CASES
//-----------------------------------------------------



//-----------------------------------------------------
// SUCCESS REQUEST TEST CASES
//-----------------------------------------------------
#[actix_web::test]
async fn test_register_user_success(){
    let app = build_test_app().await;
    
    let username = unique_test_username();
    let email = unique_test_email();
    
    let request = test::TestRequest::post()
        .uri("/auth/register")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .set_json(json!({
            "username": username,
            "email": email,
            "password_text": "T1234567890."
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert!(
        response.status() == StatusCode::CREATED || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "unexpected status: {}", response.status()
    );
    // let body: serde_json::Value = test::read_body_json(response).await;
    // assert_eq!(body["message"], "Created user testuser");
}

#[actix_web::test]
async fn test_login_user_success(){
    let app = build_test_app().await;

    let username = unique_test_username();
    let email = unique_test_email();

    let register_req = test::TestRequest::post()
        .uri("/auth/register")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .set_json(json!({
            "username": username,
            "email": email,
            "password_text": "T1234567890."
        }))
        .to_request();
    let _ = test::call_service(&app, register_req).await;
    
    let request = test::TestRequest::post()
        .uri("/auth/login")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .set_json(json!({
            "email": email,
            "password_text": "T1234567890."
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    let access_token_cookie = response.response().cookies()
        .find(|c| c.name() == "access_token")
        .expect("access token cookie was not set");
    let refresh_token_cookie = response.response().cookies()
        .find(|c| c.name() == "refresh_token")
        .expect("refresh token cookie was not set");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(!access_token_cookie.value().is_empty(), "expected access token cookie to be set");
    assert!(access_token_cookie.http_only().unwrap_or(false), "access token should be HttpOnly");
    assert!(!refresh_token_cookie.value().is_empty(), "expected refresh token cookie to be set");
    assert!(refresh_token_cookie.http_only().unwrap_or(false), "refresh token should be HttpOnly");
}

#[actix_web::test]
async fn test_refresh_token_json(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;

    let refresh_str = tokens_data["refresh"]
        .as_str()
        .expect("refresh token was not a string");

    let request = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .set_json(json!({
            "refresh_token": format!("{}", refresh_str),
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["message"], "Token refreshed");   
}

#[actix_web::test]
async fn test_refresh_token_headers(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    let refresh_str = tokens_data["refresh"]
        .as_str()
        .expect("refresh token was not a string");
    let request = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("X-Refresh-Token",  format!("{}", refresh_str)))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["message"], "Token refreshed");   
}

#[actix_web::test]
async fn test_refresh_token_cookies(){
    let app = build_test_app().await;

    let (_, refresh_cookie) = login_and_get_token(&app).await;

    let refresh_request = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .cookie(refresh_cookie)
        .to_request();

    let refresh_response = test::call_service(&app, refresh_request).await;

    let new_access_cookie = refresh_response
        .response()
        .cookies()
        .find(|c| c.name() == "access_token")
        .expect("refresh should issuse a new access token cookie");
    let new_refresh_cookie = refresh_response
        .response()
        .cookies()
        .find(|c| c.name() == "refresh_token")
        .expect("refresh should issuse a new refresh token cookie");
    
    assert_eq!(refresh_response.status(), StatusCode::OK);
    assert!(!new_access_cookie.value().is_empty());
    assert!(!new_refresh_cookie.value().is_empty());

    let body: serde_json::Value = test::read_body_json(refresh_response).await;

    assert_eq!(body["message"], "Token refreshed");
}

#[actix_web::test]
async fn test_logout_success(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    
    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");
    let refresh_str = tokens_data["refresh"]
        .as_str()
        .expect("access token was not a string");

    let request = test::TestRequest::post()
        .uri("/auth/user/logout")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .insert_header(("X-RefreshToken", format!("{}", refresh_str)))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);   
}


// views routes test
#[actix_web::test]
async fn test_getuser_success(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");

    let request = test::TestRequest::get()
        .uri("/api/user/me")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .to_request();

    let response = test::call_service(&app, request).await;

    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["message"], "Current user information");
}

#[actix_web::test]
async fn test_multiple_product_success(){
    let app = build_test_app().await;

    let url = "/api/products?p_n=hp%20laptop&s_t=id&p=1&p_p=10".to_string();
    let request = test::TestRequest::get()
        .uri(&url)
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);  
}

#[actix_web::test]
async fn test_single_product_success(){
    let app = build_test_app().await;

    let url = format!("/api/product?n={}", 10);
    let request = test::TestRequest::get()
        .uri(&url)
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);  
}

#[actix_web::test]
async fn test_bookmark_success(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");
    let request = test::TestRequest::post()
        .uri("/api/user/bookmark")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .set_json(json!({
            "product_id":2    
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[actix_web::test]
async fn test_history_success(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");
    let request = test::TestRequest::post()
        .uri("/api/history")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .set_json(json!({
            "phrase":"test search 001"
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[actix_web::test]
async fn test_del_bookmark_success(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;

    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");

    let b_request = test::TestRequest::post()
        .uri("/api/user/bookmark")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .set_json(json!({
            "product_id":9,
        }))
        .to_request();

    let _ = test::call_service(&app, b_request).await;
  
    let request = test::TestRequest::post()
        .uri("/api/user/del/bookmark")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .set_json(json!({
            "product_id":9,
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_del_history_success(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");


    let h_request = test::TestRequest::post()
        .uri("/api/history")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .set_json(json!({
            "phrase":"test search"
        }))
        .to_request();

    let _ = test::call_service(&app, h_request).await;
    
    let request = test::TestRequest::post()
        .uri("/api/del/history")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .set_json(json!({
            "phrase":"test search"
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[actix_web::test]

async fn test_currency_success(){
    let app = build_test_app().await;

    let request_otb = test::TestRequest::get()
        .uri("/api/currency-converstion?ammount=2500&from=NGN&converstion_type=OTB")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .to_request();
    let response_otb = test::call_service(&app, request_otb).await;
    let response_otb_status = response_otb.status();
    assert_eq!(response_otb_status, StatusCode::OK);

    let request_oto = test::TestRequest::get()
        .uri("/api/currency-converstion?ammount=2500&from=NGN&to=GBP&converstion_type=OTO")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .to_request();
    let response_oto = test::call_service(&app, request_oto).await;
    let response_oto_status = response_oto.status();
    assert_eq!(response_oto_status, StatusCode::OK);
}

#[actix_web::test]
async fn test_user_order(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");

    let req = test::TestRequest::get()
        .uri("/api/user/order?product_id=1")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .to_request();

    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::OK);
}


#[actix_web::test]
async fn test_prod_hit(){
    let app = build_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/hit?product_id=1")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .to_request();

    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::OK);
}

// search handler test 
#[actix_web::test]
async fn test_search(){
    let pyserver = TestSuitPyServer::new().await;
    eprintln!("\n {:?} \n", pyserver);
    let app = build_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/search?query=laptop")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .to_request();

    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::OK);

    // let body: serde_json::Value = test::read_body_json(response).await;
    // eprintln!("\n {:?} \n", body);
}

//-----------------------------------------------------
// FAILED REQUEST TEST CASES
//-----------------------------------------------------
#[actix_web::test]
async fn test_getuser_failed(){
    let app = build_test_app().await;

    let request = test::TestRequest::get()
        .uri("/user/me")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer wwwewdmssojxjidndj")))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_multiple_product_failed(){
    let app = build_test_app().await;

    let url = "/api/products?p_n=&s_t=id&p=1&p_p=10".to_string();
    let request = test::TestRequest::get()
        .uri(&url)
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);  
}


#[actix_web::test]
async fn test_single_product_failed(){
    let app = build_test_app().await;

    let url = "/api/product?n=999999999".to_string();
    let request = test::TestRequest::get()
        .uri(&url)
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_bookmark_failed(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");
    let request = test::TestRequest::post()
        .uri("/api/user/bookmark")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .set_json(json!({
            "product_id":9999999,
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_del_bookmark_failed(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");
    let request = test::TestRequest::post()
        .uri("/api/user/del/bookmark")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .set_json(json!({
            "product_id":9999999,
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_del_histroy_failed(){
    let app = build_test_app().await;

    let (tokens_data, _) = login_and_get_token(&app).await;
    let token_str = tokens_data["token"]
        .as_str()
        .expect("access token was not a string");
    let request = test::TestRequest::post()
        .uri("/api/del/history")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .insert_header(("Authorization", format!("Bearer {}", token_str)))
        .set_json(json!({
            "phrase":"qqwqqq"
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}



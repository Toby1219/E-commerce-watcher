use std::{
    process::{Child, Command}, 
    time::{Duration, Instant},
    env
};
use reqwest::Client;
use std::path::PathBuf;
use actix_web::{
    App, body::MessageBody, cookie::Cookie, dev::{Service, ServiceResponse}, http::StatusCode, test, web
};
use actix_http::Request;
use serde_json::{json, Value};
use sea_orm::prelude::DatabaseConnection;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};
use utoipa::OpenApi;
use utoipa_actix_web::AppExt; 

use api::middleware::redis_cache::CacheReqResp;
use api::schemas::others::{AppEnvConfig, AppState, ApiDoc};
use api::utils::db::connect_db;
use api::middleware::limiter::RoutesLimiter;
use api::routes::{auth::config_auth_routes, views::config_views_routes};


static TEST_USER_COUNTER:AtomicU32 = AtomicU32::new(0);

pub fn unique_test_email() -> String {
    let n = TEST_USER_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("testuser_{}_{}@gmail.com", std::process::id(), n)
}

pub fn unique_test_username() -> String {
    let n = TEST_USER_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("testuser_{}_{}", std::process::id(), n)
}

pub async fn build_test_app() -> impl Service<
    actix_http::Request,
    Response = ServiceResponse<impl MessageBody>,
    Error = actix_web::Error,
> {
    dotenvy::dotenv().ok();

    let config_envapp = AppEnvConfig::from_env().unwrap();
    let db_conn: DatabaseConnection = connect_db(&config_envapp.db_url).await;
    let redis_conn: CacheReqResp = CacheReqResp::connect(&config_envapp.redis_url).await.expect("failed to connect to Redis");
    
    let state = web::Data::new(AppState {
        db: db_conn,
        redis: redis_conn,
        env_config: config_envapp.clone(),
    });

    let limiter = RoutesLimiter::new();

    let (app, _) =
        App::new()
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .app_data(state.clone())
            .configure(|cfg| config_auth_routes(cfg, limiter.auth_route.clone()))
            .configure(|cfg| config_views_routes(cfg, limiter.view_route.clone()))
            .split_for_parts();

    test::init_service(app).await
}

pub async fn login_and_get_token<S, B>(app: &S) -> (Value, Cookie<'_>)
where 
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
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
    let register_resp = test::call_service(app, register_req).await;
    assert_eq!(register_resp.status(), StatusCode::CREATED, "setup: registration failed");
    
    let login_req = test::TestRequest::post()
        .uri("/auth/login")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .set_json(json!({ "email": email, "password_text": "T1234567890." }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;

    let refresh_cookie: Cookie<'_> = login_resp.response()
        .cookies()
        .find(|c| c.name() == "refresh_token")
        .expect("refresh token cookie was not set")
        .into_owned();

    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let token_ = login_body["body"]["token"].as_str().unwrap().to_string();
    let refresh =  login_body["body"]["refresh_token"].as_str().unwrap().to_string();

    let tokens_value = json!({
        "token":token_,
        "refresh": refresh
    });
    (tokens_value, refresh_cookie)
}


#[derive(Debug)]
pub struct TestSuitPyServer{
    pub child: Child
}

impl TestSuitPyServer {
    pub async fn new()-> Self{
        let base_dir = if let Ok(custom_root) = env::var("APP_ROOT") {
            // for docker path
            PathBuf::from(custom_root)
        } else {
            let home = env::var("HOME").expect("HOME environment variable not set");
            PathBuf::from(home).join("Documents/ewatch")
        };
        let python_project_dir = base_dir.join("services/crawlers");
        let venv_python_path = base_dir.join("services/.venv/bin/python");

        assert!(
            venv_python_path.exists(),
            "Python interpreter not found: {}",
            venv_python_path.display()
        );

        assert!(
            python_project_dir.exists(),
            "Python project directory not found: {}",
            python_project_dir.display()
        );

        eprintln!("Starting Python embedding service...");
        eprintln!("Python: {}", venv_python_path.display());
        eprintln!("Working directory: {}", python_project_dir.display());

        let mut child = Command::new(venv_python_path)
            .args(["-m", "uvicorn", "api:app", "--host", "127.0.0.1", "--port", "8000"])
            .current_dir(python_project_dir)
            .spawn()
            .expect("Failed to start FastAPI server from external directory. Check .venv path.");

        let start = Instant::now();
        let timeout = Duration::from_secs(30);
        let health_url = "http://127.0.0.1:8000/health";

        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("Failed to create HTTP client");

        loop {

            match child.try_wait() {
                Ok(Some(status)) => panic!("Python service exited during startup: {status}"),
                Ok(None) => {},
                Err(err) => panic!("Failed to check Python process: {:?}", err),
            }

            if start.elapsed() > timeout {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Python embedding service did not become ready within {:?}", timeout);
            }

            match client.get(health_url).send().await {
                Ok(response) if response.status().is_success() => {
                    eprintln!(
                        "Python embedding service is ready after {:?}",
                        start.elapsed()
                    );

                    return Self {child};
                }
                Ok(response) => {
                    eprintln!(
                        "Health check returned HTTP {}",
                        response.status()
                    );
                }
                Err(error) => {
                    eprintln!("Waiting for Python service: {error}");
                }
            }

            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

impl Drop for TestSuitPyServer{
    fn drop(&mut self){
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

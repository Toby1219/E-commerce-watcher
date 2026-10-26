use actix_web::{App, HttpServer, middleware::{Logger, NormalizePath}, web, HttpRequest, HttpResponse};
use actix_cors::Cors;
use actix_web::http::header;
use sea_orm::prelude::DatabaseConnection;
use utoipa::OpenApi;
use utoipa_actix_web::AppExt; 
use utoipa_swagger_ui::SwaggerUi; 

use api::{middleware::redis_cache::CacheReqResp, schemas::{others::ApiDoc, response_sch::ApiResponse}};
use api::utils::db::connect_db;
use api::middleware::limiter::RoutesLimiter;
use api::routes::{views::config_views_routes, auth::config_auth_routes};
use api::schemas::others::{AppEnvConfig, AppState};
use api::tasks::{cleanup_task::{spawn_token_cleanup_worker, spawn_history_cleanup_worker}, embed_task::spawn_embedding_backfill, other_tasks::run_curency_spider};

async fn fallback(request:HttpRequest)->HttpResponse {
    ApiResponse::<()>::new(                                                  
        404,                                                                 
        format!("No route for {} {}", request.method(), request.path()),    
        (),                                                                
    )
}


#[actix_web::main]
async fn main()-> std::io::Result<()>{
    dotenvy::dotenv().ok();

    let config_envapp = AppEnvConfig::from_env().unwrap();

    env_logger::init();

    let db_conn:DatabaseConnection  = connect_db(&config_envapp.db_url).await;
    let redis_conn: CacheReqResp = CacheReqResp::connect(&config_envapp.redis_url).await.expect("failed to connect to Redis");
    
    let state = web::Data::new(AppState {
        db: db_conn,
        redis: redis_conn,
        env_config: config_envapp.clone(),
    });

    spawn_token_cleanup_worker(state.db.clone());
    spawn_history_cleanup_worker(state.db.clone());
    spawn_embedding_backfill(state.clone());
    run_curency_spider(state.env_config.py_api_host.clone());

    let limiter = RoutesLimiter::new();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(config_envapp.cors_origin.as_str())
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://127.0.0.1:5173")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600);
        let (app, api) = App::new()
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .map(|app| app.wrap(cors).wrap(Logger::default()).wrap(NormalizePath::trim()))
            .app_data(state.clone())
            .configure(|cfg| config_auth_routes(cfg, limiter.auth_route.clone()))
            .configure(|cfg| config_views_routes(cfg, limiter.view_route.clone()))
            .default_service(web::route().to(fallback))
            .split_for_parts();

        app.service(
            SwaggerUi::new("/swagger-ui/{_:.*}")
            .url("/api-docs/openapi.json", api)

        )
    })
    
    .bind((config_envapp.bind_address.as_str(), config_envapp.port as u16))?
    .run()
    .await
}
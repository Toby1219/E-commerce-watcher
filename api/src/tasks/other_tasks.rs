use std::time::Duration;

use crate::utils::utils_funcs::make_request;
use crate::schemas::others::{StatusApiResponse};
use crate::schemas::response_sch::ApiError;

pub fn run_curency_spider() {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_hours(24));
        loop {
            interval.tick().await;
            log::info!("Running currency scrapy spider");
            let host = std::env::var("PY_API_HOST").expect("Invalid PY API HOST");
            let response:Result<StatusApiResponse, ApiError> = make_request(
                None, 
                "update-currency".to_string(),
                host, 
                "get".to_string()
            ).await;
            match response {
                Ok(r) => log::info!("API resonse: {:?}", r.status),
                Err(e) => {
                    log::info!("API resonse error: {:?}", e); 
                    tokio::time::sleep(Duration::from_secs(200)).await;
                },
            }
        }
            
    });
    
}
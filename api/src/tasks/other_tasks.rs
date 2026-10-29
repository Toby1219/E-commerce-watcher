use std::time::Duration;

use crate::utils::utils_funcs::make_request;
use crate::schemas::others::{StatusApiResponse};

pub fn run_curency_spider(host: String) {
    tokio::spawn(async move {
        let idle = Duration::from_hours(24);
        let retry = Duration::from_secs(60);
        loop {
            log::info!("Running currency scrapy spider");
            let response_wait = match make_request::<StatusApiResponse>(
                None, 
                "update-currency".to_string(),
                host.clone(), 
                "get".to_string()
            ).await {
                Ok(r) => {
                    log::info!("API resonse: {:?}", r.status); 
                    idle
                },
                Err(e) => {
                    log::info!("API resonse error: {:?}", e);
                    retry
                }
            };
            tokio::time::sleep(response_wait).await;
        }
            
    });
    
}
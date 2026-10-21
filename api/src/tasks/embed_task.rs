use actix_web::web;
use std::time::Duration;

use crate::schemas::others::AppState;
use crate::utils::text_embedder::backfill_product_embeddings;


pub fn spawn_embedding_backfill(state: web::Data<AppState>) {
    tokio::spawn(async move {
        let idle = Duration::from_secs(30);
        let retry = Duration::from_secs(60);
        loop {
            // interval.tick().await;
            let wait = match backfill_product_embeddings(state.clone()).await {
                Ok(n) => {
                    if n > 0 { log::info!("backfill done: {} products", n) }
                    idle
                },
                Err(e) => {
                    log::error!("backfill failed: {:?}", e);
                    retry
                },
            };
            tokio::time::sleep(wait).await;
        }
        
    });
}
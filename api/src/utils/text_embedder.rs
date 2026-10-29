use actix_web::{web};
use sea_orm::{ConnectionTrait, DbBackend, FromQueryResult, Statement};
use sea_orm::prelude::PgVector;

use crate::schemas::response_sch::ApiError;
use crate::schemas::others::{AppState, EmbedderApiResponse, EmbeddingResponseData};
use crate::schemas::others::TextPayload;
use crate::utils::utils_funcs::make_request;

#[derive(FromQueryResult)]
struct Pending { id: i64, product_name: String }


pub async fn embed_one(text: String, host:String) -> Result<Vec<f32>, ApiError> {
    let response:EmbedderApiResponse = make_request(
        Some(TextPayload::Single(text)), 
        "text-embedder".to_string(), 
        host, 
        "post".to_string()
    ).await?;
    match response.data {
        EmbeddingResponseData::Single(v) => Ok(v),
        EmbeddingResponseData::Multiple(_) => Err(ApiError::SeverError),
    }
}

pub async fn embed_many(texts: Vec<String>, host:String) -> Result<Vec<Vec<f32>>, ApiError> {
    let expected_len = texts.len();
    let response:EmbedderApiResponse = make_request(
        Some(TextPayload::Multiple(texts)),
        "text-embedder".to_string(), 
        host, 
        "post".to_string()
    ).await?;
    match response.data {
        EmbeddingResponseData::Multiple(v) if v.len() == expected_len => Ok(v),
        EmbeddingResponseData::Multiple(_) => Err(ApiError::NotFound(
            "[text embedder] returned an unexpected number of vectors".to_string(),
        )),
        EmbeddingResponseData::Single(_) => Err(ApiError::NotFound(
            "[text embedder] expected many vector, got a single one".to_string(),
        )),
    }
}


pub async fn backfill_product_embeddings(state: web::Data<AppState>) -> Result<usize, ApiError> {
    let mut total: usize = 0;

    loop {
        let batch: Vec<Pending> = Pending::find_by_statement(
            Statement::from_string(
                DbBackend::Postgres,
                "SELECT id, product_name FROM products WHERE embedding IS NULL ORDER BY id LIMIT 64",
            ),
        )
        .all(&state.db)
        .await
        .map_err(ApiError::DatabaseError)?;

        if batch.is_empty() {
            break;
        }

        let names: Vec<String> = batch.iter().map(|p| p.product_name.clone()).collect();
        let vectors = embed_many(names, state.env_config.py_api_host.clone()).await?;
        
        for (p, v) in batch.iter().zip(vectors) {
            state.db
                .execute_raw(
                    Statement::from_sql_and_values(
                        DbBackend::Postgres, 
                        "UPDATE products SET embedding = $1 WHERE id = $2", 
                        [PgVector::from(v).into(), p.id.into()]
                    )
                )
                .await
                .map_err(ApiError::DatabaseError)?;
        }

        total += batch.len();
        log::info!("embedded {} products so far", total);
    }

    Ok(total)
}


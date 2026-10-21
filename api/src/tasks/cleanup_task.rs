use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Condition};
use chrono::Utc;
use std::time::Duration;

use entities::tokens;
use entities::history;

pub fn spawn_token_cleanup_worker(db:DatabaseConnection) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(150));
        loop {
            interval.tick().await;
            log::info!("Running token cleanup worker");

            let tokens = tokens::Entity::delete_many()
                .filter(tokens::Column::ExpiredAt.lt(Utc::now()))
                .exec(&db)
                .await;

            match tokens {
                Ok(res) => log::info!("cleanup: removed {} expired tokens", res.rows_affected),
                Err(e) => log::error!("tokens cleanup worker failed: {}", e),
            };
        }
    });
}


pub fn spawn_history_cleanup_worker(db:DatabaseConnection) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(150));
        loop {
            interval.tick().await;
            log::info!("Running history cleanup worker");
            let cutoff = Utc::now() - Duration::from_hours(72); // 3 days
            let condition = Condition::all()
                .add(history::Column::UserId.is_null())
                .add(history::Column::CreatedAt.lt(cutoff));

            let history = history::Entity::delete_many()
                .filter(condition)
                .exec(&db)
                .await;

            match history {
                Ok(res) => log::info!("cleanup: removed {} expired histories", res.rows_affected),
                Err(e) => log::error!("histories cleanup worker failed: {}", e),
            };
        }
    });
}



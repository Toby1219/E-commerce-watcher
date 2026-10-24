use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261022_120000_add_products_embedding_hnsw_index"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS vector").await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS products_embedding_hnsw_idx \
             ON products \
             USING hnsw (embedding vector_cosine_ops) \
             WITH (m = 16, ef_construction = 64)",
        ).await?;
        Ok(())
    }

    // Runs on `migration down`
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS products_embedding_hnsw_idx")
            .await?;
        Ok(())
    }
}
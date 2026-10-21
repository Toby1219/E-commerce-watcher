use sea_orm_migration::{prelude::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261001_155113_vector_entity"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS vector").await?;
        db.execute_unprepared(
            "CREATE TABLE items (id serial PRIMARY KEY, content text NOT NULL, embedding vector(768) NOT NULL)",
        ).await?;
        db.execute_unprepared(
            "CREATE INDEX ON items USING hnsw (embedding vector_cosine_ops)",
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DROP TABLE items").await?;
        Ok(())
    }
}

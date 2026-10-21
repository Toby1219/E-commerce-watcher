use sea_orm_migration::{prelude::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261003_151823_create_update_product_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS vector").await?;
        db.execute_unprepared("ALTER TABLE products ADD COLUMN IF NOT EXISTS embedding vector(768)").await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection()
            .execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS embedding").await?; // undo
        Ok(())
    }
}

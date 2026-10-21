use sea_orm_migration::{prelude::*, sea_orm::{DatabaseBackend, Schema}};
use entities::currencies;


pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260910_150528_currency_model"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(DatabaseBackend::Postgres);
        manager.create_table(schema.create_table_from_entity(currencies::Entity))
        .await?;
        
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(currencies::Entity).to_owned())
        .await?;
        Ok(())
    }
}

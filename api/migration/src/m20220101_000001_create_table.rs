use sea_orm_migration::{prelude::*, sea_orm::{DatabaseBackend, Schema}};
use entities::{users, products, history, bookmark, tokens};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(DatabaseBackend::Postgres);
        manager.create_table(schema.create_table_from_entity(users::Entity))
        .await?;
        manager.create_table(schema.create_table_from_entity(products::Entity))
        .await?;
        manager.create_table(schema.create_table_from_entity(history::Entity))
        .await?;
        manager.create_table(schema.create_table_from_entity(bookmark::Entity))
        .await?;
        manager.create_table(schema.create_table_from_entity(tokens::Entity))
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(users::Entity).to_owned())
        .await?;
        manager.drop_table(Table::drop().table(products::Entity).to_owned())
        .await?;
        manager.drop_table(Table::drop().table(history::Entity).to_owned())
        .await?;
        manager.drop_table(Table::drop().table(bookmark::Entity).to_owned())
        .await?;
        manager.drop_table(Table::drop().table(tokens::Entity).to_owned())
        .await?;
        Ok(())
    }
}

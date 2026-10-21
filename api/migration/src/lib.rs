pub use sea_orm_migration::prelude::*;
mod m20220101_000001_create_table;
mod m20260910_150528_currency_model;
mod m20260916_181502_alter_table_user;
mod m20261001_155113_vector_entity;
mod m20261003_151823_create_update_product_table;
mod m20261003_230246_create_update_history;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20260910_150528_currency_model::Migration),
            Box::new(m20260916_181502_alter_table_user::Migration),
            Box::new(m20261001_155113_vector_entity::Migration),
            Box::new(m20261003_151823_create_update_product_table::Migration),
            Box::new(m20261003_230246_create_update_history::Migration),
        ]
    }
}

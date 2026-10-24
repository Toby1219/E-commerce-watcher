pub use sea_orm_migration::prelude::*;
mod m20220101_000001_create_table;
mod m20261001_155113_vector_entity;
mod m20261003_151823_create_update_product_table;
mod m20261022_120000_add_products_embedding_hnsw_index;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20261001_155113_vector_entity::Migration),
            Box::new(m20261003_151823_create_update_product_table::Migration),
            Box::new(m20261022_120000_add_products_embedding_hnsw_index::Migration),
        ]
    }
}

pub use sea_orm_migration::prelude::*;
mod create_tables;
mod create_update_product_table;
mod add_products_embedding_hnsw_index;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(create_tables::Migration),
            Box::new(create_update_product_table::Migration),
            Box::new(add_products_embedding_hnsw_index::Migration),
        ]
    }
}

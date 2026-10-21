use sea_orm::entity::prelude::*; 
use sea_orm::prelude::PgVector;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "items")] 
pub struct Model {
    #[sea_orm(primary_key)] 
    pub id: i32,
    pub content: String, 
    pub embedding: PgVector,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {} 

impl ActiveModelBehavior for ActiveModel {} 



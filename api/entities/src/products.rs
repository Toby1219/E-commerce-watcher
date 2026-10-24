use sea_orm::prelude::*;
use sea_orm::prelude::PgVector;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name="products")]
pub struct Model{
    #[sea_orm(primary_key)]
    pub id: i64,
    pub product_url: String,
    pub product_name: String,
    pub product_price: Decimal,
    pub product_price_text: String,
    pub product_currency: String,
    pub product_brand_name: Option<String>,
    pub product_category: Option<String>,
    pub product_image:Option<String>,
    pub product_sku: Option<String>,
    #[sea_orm(column_type="JsonBinary")]
    pub product_details: Value,
    #[sea_orm(column_type="JsonBinary")]
    pub product_reviews: Value,
    #[sea_orm(column_type="JsonBinary")]
    pub product_specifications: Value,
    pub score: i64,
    pub created_at: DateTime<Utc>,
    pub embedding: Option<PgVector>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::bookmark::Entity")]
    Bookmark
}

impl Related<super::bookmark::Entity> for Entity {
    fn to()->RelationDef{
        Relation::Bookmark.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}



use sea_orm::prelude::*;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name="currencies")]
pub struct Model{
    #[sea_orm(primary_key)]
    pub id: i64,
    pub base_currency: String,
    pub currency: String,
    pub rates: Decimal,
    pub time_last_update_utc:DateTime<Utc>,
    pub time_next_update_utc: DateTime<Utc>
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
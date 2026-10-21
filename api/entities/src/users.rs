use sea_orm::entity::prelude::*;
use chrono::{Utc, DateTime};

#[derive(Clone, PartialEq, Eq, Debug, DeriveEntityModel)]
#[sea_orm(table_name="user_account")]
pub struct Model{
    #[sea_orm(primary_key)]
    pub id: i64,

    #[sea_orm(unique, indexed)]
    pub email: String,

    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub joined: DateTime<Utc>,
    pub orders_count:Option<i64>

}

#[derive(Clone, Copy, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::history::Entity")]
    History,

    #[sea_orm(has_one = "super::bookmark::Entity")]
    Bookmark
}

impl Related<super::history::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::History.def()
    }
}

impl Related<super::bookmark::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Bookmark.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
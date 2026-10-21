use sea_orm::entity::prelude::*;
use chrono::{Utc, DateTime};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name="history")]
pub struct Model{
    #[sea_orm(primary_key)]
    pub id: i64,
    pub phrase: String,
    pub user_id: Option<i64>,
    
    #[sea_orm(unique, indexed)]
    pub anon_user: Option<String>,
    
    pub created_at: DateTime<Utc>
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade",
    )]
    Users
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef{
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
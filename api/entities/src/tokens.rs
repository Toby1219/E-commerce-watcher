use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tokens")]
pub struct Model{
    #[sea_orm(primary_key)]
    pub id: i64,

    pub token: String,
    
    pub hash_token: String,
    pub revoked: bool,
    pub user_id: i64,
    pub expired_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to="super::users::Entity",
        from="Column::UserId",
        to="super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade",
    )]
    Users
}

impl Related<super::users::Entity> for Entity{
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
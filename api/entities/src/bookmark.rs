use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name="bookmarks")]
pub struct Model{
    #[sea_orm(primary_key)]
    pub id: i64,
    
    pub product_id: i64,
    pub user_id: i64,
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
    Users,

    #[sea_orm(
        belongs_to="super::products::Entity",
        from="Column::ProductId",
        to="super::products::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade",
    )]
    Products    
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}


impl Related<super::products::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Products.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
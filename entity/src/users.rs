//! SeaORM Entity. Generated by sea-orm-codegen 0.9.3

use super::sea_orm_active_enums::UserRole;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "users"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Username,
    Password,
    Role,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = i32;
    fn auto_increment() -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    ChannelUsers,
    UsersItems,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::Integer.def(),
            Self::Username => ColumnType::String(Some(512u32)).def().unique(),
            Self::Password => ColumnType::String(Some(512u32)).def(),
            Self::Role => UserRole::db_type(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::ChannelUsers => Entity::has_many(super::channel_users::Entity).into(),
            Self::UsersItems => Entity::has_many(super::users_items::Entity).into(),
        }
    }
}

impl Related<super::channel_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChannelUsers.def()
    }
}

impl Related<super::users_items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UsersItems.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

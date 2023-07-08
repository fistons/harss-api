//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "users_items"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize, Deserialize)]
pub struct Model {
    pub user_id: i32,
    pub item_id: i32,
    pub channel_id: i32,
    pub read: bool,
    pub starred: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    UserId,
    ItemId,
    ChannelId,
    Read,
    Starred,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    UserId,
    ItemId,
    ChannelId,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = (i32, i32, i32);
    fn auto_increment() -> bool {
        false
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Channels,
    Items,
    Users,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::UserId => ColumnType::Integer.def(),
            Self::ItemId => ColumnType::Integer.def(),
            Self::ChannelId => ColumnType::Integer.def(),
            Self::Read => ColumnType::Boolean.def(),
            Self::Starred => ColumnType::Boolean.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Channels => Entity::belongs_to(super::channels::Entity)
                .from(Column::ChannelId)
                .to(super::channels::Column::Id)
                .into(),
            Self::Items => Entity::belongs_to(super::items::Entity)
                .from(Column::ItemId)
                .to(super::items::Column::Id)
                .into(),
            Self::Users => Entity::belongs_to(super::users::Entity)
                .from(Column::UserId)
                .to(super::users::Column::Id)
                .into(),
        }
    }
}

impl Related<super::channels::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Channels.def()
    }
}

impl Related<super::items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

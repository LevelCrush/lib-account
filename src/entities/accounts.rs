//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.14

use sea_orm::entity::prelude::*;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "accounts"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Default)]
pub struct Model {
    pub id: i64,
    pub token: String,
    pub token_secret: String,
    pub admin: i8,
    pub timezone: String,
    pub last_login_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Token,
    TokenSecret,
    Admin,
    Timezone,
    LastLoginAt,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = i64;
    fn auto_increment() -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    AccountPlatformData,
    AccountPlatforms,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigInteger.def(),
            Self::Token => ColumnType::Char(Some(32u32)).def().unique(),
            Self::TokenSecret => ColumnType::Char(Some(32u32)).def(),
            Self::Admin => ColumnType::TinyInteger.def(),
            Self::Timezone => ColumnType::String(Some(32u32)).def(),
            Self::LastLoginAt => ColumnType::BigInteger.def(),
            Self::CreatedAt => ColumnType::BigInteger.def(),
            Self::UpdatedAt => ColumnType::BigInteger.def(),
            Self::DeletedAt => ColumnType::BigInteger.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::AccountPlatformData => {
                Entity::has_many(super::account_platform_data::Entity).into()
            }
            Self::AccountPlatforms => Entity::has_many(super::account_platforms::Entity).into(),
        }
    }
}

impl Related<super::account_platform_data::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccountPlatformData.def()
    }
}

impl Related<super::account_platforms::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccountPlatforms.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

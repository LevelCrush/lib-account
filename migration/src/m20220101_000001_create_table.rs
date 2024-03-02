use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Accounts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Accounts::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Accounts::Token)
                            .char_len(32)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Accounts::TokenSecret)
                            .char_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Accounts::Admin)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Accounts::Timezone).not_null())
                    .col(ColumnDef::new(Accounts::LastLoginAt).big_integer())
                    .col(ColumnDef::new(Accounts::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Accounts::UpdatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Accounts::DeletedAt).big_integer().not_null())
                    .index(
                        Index::create()
                            .if_not_exists()
                            .name("accounts-token-tokensecret")
                            .table(Accounts::Table)
                            .col(Accounts::Token)
                            .col(Accounts::TokenSecret)
                            .unique(),
                    )
                    .index(
                        Index::create()
                            .if_not_exists()
                            .name("accounts-admin")
                            .table(Accounts::Table)
                            .col(Accounts::Admin),
                    )
                    .to_owned(),
            )
            .await;

        manager
            .create_table(
                Table::create()
                    .table(AccountPlatforms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AccountPlatforms::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(AccountPlatforms::Platform).text().not_null())
                    .col(
                        ColumnDef::new(AccountPlatforms::Token)
                            .char_len(32)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(AccountPlatforms::PlatformUser)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AccountPlatforms::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AccountPlatforms::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AccountPlatforms::DeletedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .index(
                        Index::create()
                            .if_not_exists()
                            .table(AccountPlatforms::Table)
                            .name("accountplatforms-account-platform")
                            .col(AccountPlatforms::Account)
                            .col(AccountPlatforms::Platform)
                            .unique(),
                    )
                    .index(
                        Index::create()
                            .if_not_exists()
                            .table(AccountPlatforms::Table)
                            .name("accountplatforms-platform")
                            .col(AccountPlatforms::Platform),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(AccountPlatforms::Table, AccountPlatforms::Account)
                            .to(Accounts::Table, Accounts::Id),
                    ),
            )
            .await;

        manager
            .create_table(
                Table::create()
                    .table(AccountPlatformData::Table)
                    .col(
                        ColumnDef::new(AccountPlatformData::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AccountPlatformData::Account)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AccountPlatformData::Platform)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AccountPlatformData::Key)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(ColumnDef::new(AccountPlatformData::Value).text().not_null())
                    .index(
                        Index::create()
                            .name("apdata-account-platform")
                            .table(AccountPlatformData::Table)
                            .col(AccountPlatformData::Account)
                            .col(AccountPlatformData::Platform),
                    )
                    .index(
                        Index::create()
                            .name("apdata-account-platform-data-key")
                            .table(AccountPlatformData::Table)
                            .col(AccountPlatformData::Account)
                            .col(AccountPlatformData::Platform)
                            .col(AccountPlatformData::Key),
                    )
                    .index(
                        Index::create()
                            .name("apdata-key")
                            .table(AccountPlatformData::Table)
                            .col(AccountPlatformData::Key),
                    )
                    .index(
                        Index::create()
                            .name("apdata-platform")
                            .table(AccountPlatformData::Table)
                            .col(AccountPlatformData::Platform),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(AccountPlatformData::Table, AccountPlatformData::Account)
                            .to(Accounts::Table, Accounts::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(AccountPlatformData::Platform, AccountPlatformData::Platform)
                            .to(AccountPlatforms::Table, AccountPlatforms::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AccountPlatformData::Table).to_owned())
            .await;

        manager
            .drop_table(Table::drop().table(AccountPlatforms::Table).to_owned())
            .await;

        manager
            .drop_table(Table::drop().table(Accounts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
    Token,
    TokenSecret,
    Timezone,
    Admin,
    LastLoginAt,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum AccountPlatforms {
    Table,
    Id,
    Account,
    Token,
    Platform,
    PlatformUser,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum AccountPlatformData {
    Table,
    Id,
    Account,
    Platform,
    Key,
    Value,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

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
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Accounts::TokenSecret).text().not_null())
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

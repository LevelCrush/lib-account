create table if not exists account_platform_data
(
    id         INTEGER not null
        constraint account_platform_data_pk
            primary key autoincrement,
    account    INTEGER not null,
    platform   INTEGER not null,
    key        TEXT    not null,
    value      TEXT    not null,
    created_at INTEGER not null,
    updated_at INTEGER not null,
    deleted_at INTEGER,
    constraint account_platform_data_pk2
        unique (platform, account, key)
)
    strict;

create index if not exists account_platform_data_account_index
    on account_platform_data (account);

create index if not exists account_platform_data_key_index
    on account_platform_data (key);

create index if not exists account_platform_data_key_platform_index
    on account_platform_data (key, platform);

create index if not exists account_platform_data_platform_index
    on account_platform_data (platform);

create index if not exists main.account_platform_data_value_index
    on account_platform_data (value);

create table if not exists account_platforms
(
    id            INTEGER not null
        constraint account_platforms_pk
            primary key autoincrement,
    account       INTEGER not null,
    token         TEXT    not null
        constraint account_platforms_pk2
            unique,
    platform      TEXT    not null,
    platform_user TEXT    not null,
    created_at    INTEGER not null,
    updated_at    INTEGER not null,
    deleted_at    INTEGER not null
)
    strict;

create index if not exists account_platforms_account_index
    on account_platforms (account);

create index if not exists account_platforms_account_platform_index
    on account_platforms (account, platform);

create index if not exists account_platforms_platform_index
    on account_platforms (platform);

create index if not exists account_platforms_platform_user_index
    on account_platforms (platform_user);

create table if not exists accounts
(
    id            integer not null
        constraint accounts_pk
            primary key autoincrement,
    token         TEXT    not null
        constraint accounts_token_unique
            unique,
    token_secret  TEXT    not null
        constraint accounts_token_secret_unique
            unique,
    timezone      TEXT    not null,
    admin         INTEGER not null,
    last_login_at integer not null,
    created_at    integer not null,
    updated_at    integer not null,
    deleted_at    integer not null
)
    strict;

create index if not exists accounts_admin_index
    on accounts (admin);

create index if not exists accounts_timezone_index
    on accounts (timezone);


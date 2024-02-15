use levelcrush::{
    database,
    macros::{DatabaseRecord, DatabaseResult},
    md5, project_str,
    util::unix_timestamp,
};
use sqlx::SqlitePool;
use std::collections::HashMap;

#[DatabaseResult]
pub struct AccountLinkedPlatformsResult {
    pub account_token: String,
    pub username: String,
    pub discord: String,
    pub bungie: String,
    pub twitch: String,
}

#[DatabaseRecord]
pub struct Account {
    pub token: String,
    pub admin: i64,
    pub token_secret: String,
    pub timezone: String,
    pub last_login_at: i64,
}

pub async fn get<T: Into<String>, TS: Into<String>>(
    token: T,
    token_secret: TS,
    pool: &SqlitePool,
) -> Option<Account> {
    let token = token.into();
    let token_secret = token_secret.into();

    let query_result = sqlx::query_file_as!(
        Account,
        "queries/account_get_by_token.sql",
        token,
        token_secret
    )
    .fetch_optional(pool)
    .await;

    if let Ok(query_result) = query_result {
        query_result
    } else {
        database::log_error(query_result);
        None
    }
}

/**
 * @brief Inserts and returns the account that is created based off the two provided seeds
 *
 * * `token_seed` Seed used to compute the public token identifier.
 * * `token_secret_seed` Seed used to compute the private token identifier
 */
pub async fn create<TokenSeed: Into<String>, TokenSecretSeed: Into<String>>(
    token_seed: TokenSeed,
    token_secret_seed: TokenSecretSeed,
    pool: &SqlitePool,
) -> Option<Account> {
    let token = format!("{:x}", md5::compute(token_seed.into()));
    let token_secret = format!("{:x}", md5::compute(token_secret_seed.into()));
    let timestamp = unix_timestamp();

    let query_result =
        sqlx::query_file!("queries/account_insert.sql", token, token_secret, timestamp)
            .execute(pool)
            .await;

    // if we were able to insert a new user fetch it based off the last inserted id from our query result
    let mut user = None;
    if let Ok(query_result) = query_result {
        let last_inserted_id = query_result.last_insert_rowid();
        let account_result =
            sqlx::query_file_as!(Account, "queries/account_get_by_id.sql", last_inserted_id)
                .fetch_optional(pool)
                .await;

        if let Ok(account_result) = account_result {
            user = account_result;
        } else {
            database::log_error(account_result);
        }
    } else {
        database::log_error(query_result);
    }

    user
}

pub async fn all_data(
    account: &Account,
    pool: &SqlitePool,
) -> HashMap<String, HashMap<String, String>> {
    let query_results = sqlx::query_file!("queries/account_platform_all_data.sql", account.id)
        .fetch_all(pool)
        .await;

    // loop through the data and construct a hashmap of those values aggregated
    let mut results = HashMap::new();
    if query_results.is_ok() {
        let query_results = query_results.unwrap();
        for record in query_results.into_iter() {
            //let index = format!("{}@{}", record.platform, record.platform_user);
            let index = record.platform;
            if !results.contains_key(&index) {
                results.insert(index.clone(), HashMap::new());
            }

            results
                .entry(index)
                .and_modify(|item: &mut HashMap<String, String>| {
                    item.insert(record.key, record.value);
                });
        }
    }
    results
}

pub async fn by_bungie_bulk(
    bungie_ids: &[String],
    pool: &SqlitePool,
) -> Vec<AccountLinkedPlatformsResult> {
    let prepared_pos = vec!["?"; bungie_ids.len()].join(",");
    let statement = project_str!("queries/account_search_by_bungie_bulk.sql", prepared_pos);
    let mut query_builder = sqlx::query_as::<_, AccountLinkedPlatformsResult>(statement.as_str());
    for bungie_id in bungie_ids.iter() {
        query_builder = query_builder.bind(bungie_id);
    }

    let query = query_builder.fetch_all(pool).await;
    if let Ok(query) = query {
        query
    } else {
        database::log_error(query);
        Vec::new()
    }
}

pub async fn by_bungie(
    bungie_id: String,
    pool: &SqlitePool,
) -> Option<AccountLinkedPlatformsResult> {
    let query = sqlx::query_file_as!(
        AccountLinkedPlatformsResult,
        "queries/account_search_by_bungie.sql",
        bungie_id
    )
    .fetch_optional(pool)
    .await;

    if let Ok(query) = query {
        query
    } else {
        database::log_error(query);
        None
    }
}

pub async fn by_discord(
    discord_handle: String,
    pool: &SqlitePool,
) -> Option<AccountLinkedPlatformsResult> {
    let query = sqlx::query_file_as!(
        AccountLinkedPlatformsResult,
        "queries/account_search_by_discord.sql",
        discord_handle
    )
    .fetch_optional(pool)
    .await;

    if let Ok(query) = query {
        query
    } else {
        database::log_error(query);
        None
    }
}

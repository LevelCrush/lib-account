use levelcrush::app::ApplicationState;
use levelcrush::project_str;
use levelcrush::{database, md5, util::unix_timestamp};
use sea_orm::{
    self, ActiveModelTrait, ActiveValue, ColumnTrait, Condition, DatabaseBackend, EntityTrait, FromQueryResult,
    JoinType, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Select, Statement, Value, Values,
};
use std::collections::HashMap;

use crate::app::extension::AccountExtension;
use crate::entities::{account_platform_data, account_platforms, accounts};

#[derive(Clone, Debug, Default, serde::Serialize, FromQueryResult)]
pub struct AccountLinkedPlatformsResult {
    pub account_token: String,
    pub username: String,
    pub discord: String,
    pub bungie: String,
    pub twitch: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, FromQueryResult)]
pub struct AccountLinkedPlatformDataResult {
    pub platform: String,
    pub platform_user: String,
    pub key: String,
    pub value: String,
}

pub type Account = accounts::Model;

pub async fn get(token: &str, token_secret: &str, state: &ApplicationState<AccountExtension>) -> Option<Account> {
    let model = accounts::Entity::find()
        .filter(
            Condition::all()
                .add(accounts::Column::Token.eq(token))
                .add(accounts::Column::TokenSecret.eq(token_secret)),
        )
        .one(&state.database)
        .await;

    if let Ok(model) = model {
        model
    } else {
        database::log_error(model);
        None
    }
}

/// Inserts and returns the account that is created based off the two provided seeds
///
/// `token_seed` Seed used to compute the public token identifier.
/// `token_secret_seed` Seed used to compute the private token identifier
pub async fn create(
    token_seed: &str,
    token_secret_seed: &str,
    state: &ApplicationState<AccountExtension>,
) -> Option<Account> {
    let token = format!("{:x}", md5::compute(token_seed));
    let token_secret = format!("{:x}", md5::compute(token_secret_seed));
    let timestamp = unix_timestamp();

    let active = accounts::ActiveModel {
        id: ActiveValue::NotSet,
        token: ActiveValue::Set(token.clone()),
        token_secret: ActiveValue::Set(token_secret.clone()),
        admin: ActiveValue::Set(0),
        timezone: ActiveValue::Set("".to_string()),
        last_login_at: ActiveValue::Set(0),
        created_at: ActiveValue::Set(timestamp),
        updated_at: ActiveValue::Set(0),
        deleted_at: ActiveValue::Set(0),
    };

    let query_result = accounts::Entity::insert(active).exec(&state.database).await;
    if let Ok(query_result) = query_result {
        let last_inserted_id = query_result.last_insert_id;

        let model = accounts::Entity::find_by_id(last_inserted_id)
            .one(&state.database)
            .await;
        if let Ok(model) = model {
            model
        } else {
            database::log_error(model);
            None
        }
    } else {
        database::log_error(query_result);
        None
    }
}

/// gets all platform data tied to an account
pub async fn all_data(
    account: &Account,
    state: &ApplicationState<AccountExtension>,
) -> HashMap<String, HashMap<String, String>> {
    let query_results = account_platform_data::Entity::find()
        .select_only()
        .column(account_platforms::Column::Platform)
        .column(account_platforms::Column::PlatformUser)
        .column(account_platform_data::Column::Key)
        .column(account_platform_data::Column::Value)
        .join(
            JoinType::InnerJoin,
            account_platform_data::Relation::AccountPlatforms.def(),
        )
        .join(JoinType::InnerJoin, account_platform_data::Relation::Accounts.def())
        .filter(Condition::all().add(account_platform_data::Column::Account.eq(account.id)))
        .order_by_asc(account_platforms::Column::Platform)
        .order_by_asc(account_platforms::Column::Id)
        .order_by_asc(account_platform_data::Column::Key)
        .into_model::<AccountLinkedPlatformDataResult>()
        .all(&state.database)
        .await;

    let mut results = HashMap::new();
    if let Ok(query_results) = query_results {
        for record in query_results.into_iter() {
            let index = record.platform;
            if !results.contains_key(&index) {
                results.insert(index.clone(), HashMap::new());
            }

            results.entry(index).and_modify(|item: &mut HashMap<String, String>| {
                item.insert(record.key, record.value);
            });
        }
    } else {
        database::log_error(query_results);
    }

    results
}

pub async fn by_bungie_bulk(
    bungie_ids: &[String],
    state: &ApplicationState<AccountExtension>,
) -> Vec<AccountLinkedPlatformsResult> {
    let prepared_pos = vec!["?"; bungie_ids.len()].join(",");
    let mut binds = Vec::with_capacity(bungie_ids.len() + 1);
    for bungie_id in bungie_ids.iter() {
        binds.push(Value::String(Some(Box::new(bungie_id.clone()))));
    }

    let query = AccountLinkedPlatformsResult::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::MySql,
        project_str!("queries/account_search_by_bungie_bulk.sql", prepared_pos),
        binds,
    ))
    .all(&state.database)
    .await;

    if let Ok(query) = query {
        query
    } else {
        database::log_error(query);
        Vec::new()
    }
}

pub async fn by_bungie(
    bungie_id: String,
    state: &ApplicationState<AccountExtension>,
) -> Option<AccountLinkedPlatformsResult> {
    let query = AccountLinkedPlatformsResult::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::MySql,
        project_str!("queries/account_search_by_bungie.sql"),
        vec![Value::String(Some(Box::new(bungie_id)))],
    ))
    .one(&state.database)
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
    state: &ApplicationState<AccountExtension>,
) -> Option<AccountLinkedPlatformsResult> {
    let query = AccountLinkedPlatformsResult::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::MySql,
        project_str!("queries/account_search_by_discord.sql"),
        vec![Value::String(Some(Box::new(discord_handle)))],
    ))
    .one(&state.database)
    .await;

    if let Ok(query) = query {
        query
    } else {
        database::log_error(query);
        None
    }
}

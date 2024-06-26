use crate::app::extension::AccountExtension;
use crate::database::platform::AccountPlatform;
use crate::entities::{account_platform_data, account_platforms, accounts};
use levelcrush::app::ApplicationState;
use levelcrush::database;
use levelcrush::util::unix_timestamp;
use levelcrush::{alias::RecordId, project_str, tracing};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter,
    QuerySelect, Statement, Value,
};
use std::collections::HashMap;

#[derive(Clone, Debug, FromQueryResult)]
pub struct AccountPlatformDataSlim {
    pub id: i64,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct NewAccountPlatformData {
    pub key: String,
    pub value: String,
}

pub async fn read(
    account_platform: &AccountPlatform,
    keys: &[&str],
    state: &ApplicationState<AccountExtension>,
) -> HashMap<String, RecordId> {
    let mut results = HashMap::new();
    for key in keys.iter() {
        results.insert(key.to_string(), 0);
    }

    // convert to vector
    let keys = keys.to_vec();

    let query_result = account_platform_data::Entity::find()
        .select_only()
        .column(account_platform_data::Column::Id)
        .column(account_platform_data::Column::Key)
        .filter(
            Condition::all()
                .add(accounts::Column::Id.eq(account_platform.account))
                .add(
                    account_platforms::Column::Id
                        .eq(account_platform.id)
                        .add(account_platform_data::Column::Key.is_in(keys)),
                ),
        )
        .into_model::<AccountPlatformDataSlim>()
        .all(&state.database)
        .await;

    if query_result.is_ok() {
        let query_result = query_result.unwrap_or_default();
        for record in query_result.iter() {
            results
                .entry(record.key.clone())
                .and_modify(|record_id| *record_id = record.id);
        }
    } else {
        database::log_error(query_result);
    }
    results
}

pub async fn write(
    account_platform: &AccountPlatform,
    values: &[NewAccountPlatformData],
    state: &ApplicationState<AccountExtension>,
) {
    // get all keys we need to work with and at the same time construct a hash map that represents the key/value pairs we want to link
    let mut keys = Vec::new();
    let mut value_map = HashMap::new();
    let mut query_parameters = Vec::new();
    let mut query_values = Vec::new();
    let timestamp = unix_timestamp();
    for (index, new_data) in values.iter().enumerate() {
        keys.push(new_data.key.as_str());
        value_map.insert(new_data.key.clone(), index);

        query_parameters.push("(?,?,?,?,?,?,?)");

        query_values.extend(
            vec![
                Value::BigInt(Some(account_platform.account)),
                Value::BigInt(Some(account_platform.id)),
                Value::String(Some(Box::new(new_data.key.clone()))),
                Value::String(Some(Box::new(new_data.value.clone()))),
                Value::BigInt(Some(timestamp)),
                Value::BigInt(Some(0)),
                Value::BigInt(Some(0)),
            ]
            .into_iter(),
        );
    }

    let query_parameters = query_parameters.join(", ");
    let insert_statement =
        project_str!("queries/account_platform_data_insert.sql", query_parameters);
    //  pull in the existing data related   l to the specified account platform. We will use this to merge and figure out which are new or need to be updated

    let query = state
        .database
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::MySql,
            insert_statement,
            query_values,
        ))
        .await;

    // finally execute the query to update/insert this data
    database::log_error(query);
}

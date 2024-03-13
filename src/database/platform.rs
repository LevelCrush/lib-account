use crate::app::extension::AccountExtension;
use crate::database::account::Account;
use crate::entities::{account_platform_data, account_platforms, accounts};
use levelcrush::alias::RecordId;
use levelcrush::app::ApplicationState;
use levelcrush::util::unix_timestamp;
use levelcrush::{database, md5};
use migration::CaseStatementCondition;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, EntityTrait, Iterable, JoinType,
    QueryFilter, QuerySelect, RelationTrait,
};

pub enum AccountPlatformType {
    Discord,
    Twitch,
    Bungie,
}

impl std::fmt::Display for AccountPlatformType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountPlatformType::Discord => {
                write!(f, "discord")
            }
            AccountPlatformType::Twitch => {
                write!(f, "twitch")
            }
            AccountPlatformType::Bungie => {
                write!(f, "bungie")
            }
        }
    }
}

/// Required data inputs to generate a platform record
pub struct NewAccountPlatform {
    pub account: RecordId,
    pub platform: AccountPlatformType,
    pub platform_user: String,
}

pub type AccountPlatform = account_platforms::Model;

/// Inserts a new accounts_platform record based on provided information.
pub async fn create(
    new_platform: NewAccountPlatform,
    state: &ApplicationState<AccountExtension>,
) -> Option<AccountPlatform> {
    let token_seed = format!(
        "{}||{}||{}",
        new_platform.platform,
        new_platform.platform_user.clone(),
        unix_timestamp()
    );
    let token = format!("{:x}", md5::compute(token_seed));
    let platform = new_platform.platform.to_string();
    let platform_user = new_platform.platform_user;
    let timestamp = unix_timestamp();

    let active = account_platforms::ActiveModel {
        id: ActiveValue::NotSet,
        platform: ActiveValue::Set(platform),
        account: ActiveValue::Set(new_platform.account),
        token: ActiveValue::Set(token),
        platform_user: ActiveValue::Set(platform_user),
        created_at: ActiveValue::Set(timestamp),
        updated_at: ActiveValue::Set(0),
        deleted_at: ActiveValue::Set(0),
    };

    let query_result = account_platforms::Entity::insert(active)
        .exec(&state.database)
        .await;

    // attempt to fetch the last inserted platform record
    if let Ok(query_result) = query_result {
        let last_inserted_id = query_result.last_insert_id;

        let platform_result = account_platforms::Entity::find_by_id(last_inserted_id)
            .one(&state.database)
            .await;

        if let Ok(platform_result) = platform_result {
            platform_result
        } else {
            database::log_error(platform_result);
            None
        }
    } else {
        database::log_error(query_result);
        None
    }
}

/// fetches an account platform directly tied to the provided account and platform type
pub async fn from_account(
    account: &Account,
    platform_type: AccountPlatformType,
    state: &ApplicationState<AccountExtension>,
) -> Option<AccountPlatform> {
    let platform = platform_type.to_string();

    let query_result = account_platforms::Entity::find()
        .select_only()
        .columns(account_platforms::Column::iter())
        .join(
            JoinType::InnerJoin,
            account_platforms::Relation::Accounts.def(),
        )
        .filter(
            Condition::all()
                .add(accounts::Column::Id.eq(account.id))
                .add(account_platforms::Column::DeletedAt.eq(0))
                .add(account_platforms::Column::Platform.eq(platform)),
        )
        .one(&state.database)
        .await;

    if let Ok(query_result) = query_result {
        query_result
    } else {
        database::log_error(query_result);
        None
    }
}

/// Based off the provided platform information, attempts to match a platform login with an existing account
pub async fn match_account(
    platform_user: String,
    platform_type: AccountPlatformType,
    state: &ApplicationState<AccountExtension>,
) -> Option<Account> {
    let platform = platform_type.to_string();

    let query_result = account_platforms::Entity::find()
        .select_only()
        .columns(accounts::Column::iter())
        .join(
            JoinType::InnerJoin,
            account_platforms::Relation::Accounts.def(),
        )
        .filter(
            Condition::all()
                .add(account_platforms::Column::Platform.eq(&platform))
                .add(account_platforms::Column::PlatformUser.eq(&platform_user)),
        )
        .into_model::<accounts::Model>()
        .one(&state.database)
        .await;

    if let Ok(query_result) = query_result {
        query_result
    } else {
        database::log_error(query_result);
        None
    }
}

/// read a platform record tied to the platform user, fetches the first created linked platform that matches the provided options
pub async fn read(
    platform_type: AccountPlatformType,
    platform_user: String,
    state: &ApplicationState<AccountExtension>,
) -> Option<AccountPlatform> {
    let platform = platform_type.to_string();

    let query_result = account_platforms::Entity::find()
        .filter(
            Condition::all()
                .add(account_platforms::Column::Platform.eq(&platform))
                .add(account_platforms::Column::PlatformUser.eq(&platform_user)),
        )
        .one(&state.database)
        .await;

    if let Ok(query_result) = query_result {
        query_result
    } else {
        database::log_error(query_result);
        None
    }
}

/// Update the provied account platform record and returns a new updated account platform record
pub async fn update(
    account_platform: &mut AccountPlatform,
    state: &ApplicationState<AccountExtension>,
) -> Option<AccountPlatform> {
    // force the platform record to have an updated timestamp of modification
    account_platform.updated_at = unix_timestamp();

    // this is absolutely redudant. Unfortuntaely due to how the code was PRE
    // sea orm migration, this is the simplest and quickest solution to get the changes in
    let mut active: account_platforms::ActiveModel = account_platform.clone().into();
    active.account = ActiveValue::Set(account_platform.account);
    active.updated_at = ActiveValue::Set(account_platform.updated_at);

    // save into the database
    // we literally dont care if it is able to make the save or not
    let _ = active.save(&state.database).await;

    let query = account_platforms::Entity::find_by_id(account_platform.id)
        .one(&state.database)
        .await;

    if let Ok(query) = query {
        query
    } else {
        database::log_error(query);
        None
    }
}

/// Unlink an account platfrom by directly deleting the related data tied to the account platform and then remove the account platform record itself as well
/// This is a permanent operation
pub async fn unlink(
    account_platform: &AccountPlatform,
    state: &ApplicationState<AccountExtension>,
) {
    // delete platform data first
    let _ = account_platform_data::Entity::delete_many()
        .filter(account_platform_data::Column::Platform.eq(account_platform.id))
        .exec(&state.database)
        .await;

    // remove the account platform now
    let _ = account_platforms::Entity::delete_by_id(account_platform.id)
        .exec(&state.database)
        .await;
}

pub async fn need_update(
    platform: AccountPlatformType,
    limit: i64,
    pool: &SqlitePool,
) -> Vec<String> {
    let platform = platform.to_string();
    let query = sqlx::query_file!("queries/account_platform_need_update.sql", platform, limit)
        .fetch_all(pool)
        .await;

    if let Ok(query) = query {
        query
            .into_iter()
            .map(|record| record.discord_id)
            .collect::<Vec<String>>()
    } else {
        database::log_error(query);
        Vec::new()
    }
}

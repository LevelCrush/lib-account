use crate::{
    database::{
        self,
        platform::{AccountPlatformType, NewAccountPlatform},
        platform_data::NewAccountPlatformData,
    },
    routes::responses::DiscordUserResponse,
};
use levelcrush::{tokio, tracing, util::unix_timestamp, uuid::Uuid};
use sqlx::SqlitePool;

#[derive(Default, Clone, Debug)]
pub struct MemberSyncResult {
    pub account_token: String,
    pub account_token_secret: String,
    pub display_name: String,
    pub username: String,
}

/// Syncs the api response from discord and returns a member sync result
pub async fn member(discord_user: DiscordUserResponse, pool: &SqlitePool) -> Option<MemberSyncResult> {
    let discord_user_id = discord_user.id.unwrap_or_default();
    let mut account =
        database::platform::match_account(discord_user_id.clone(), AccountPlatformType::Discord, pool).await;
    let mut sync_result = MemberSyncResult::default();
    let new_account = if account.is_none() {
        // new account
        // no account found. Let's create an account first
        let timestamp = unix_timestamp();
        let token_seed = format!(
            "{}||{}||{}||{}",
            timestamp,
            discord_user_id.clone(),
            discord_user.discriminator.clone(),
            Uuid::new_v4(),
        );
        let token_secret_seed = format!("..{}..||..{}..||..{}..", token_seed.clone(), Uuid::new_v4(), timestamp);

        // create an account for this
        tracing::info!("Creating account");
        account = database::account::create(token_seed, token_secret_seed, pool).await;

        true
    } else {
        false
    };

    let mut account_platform = None;
    if let Some(account) = account {
        if new_account {
            tracing::info!("New account setup and being linked");
            account_platform = database::platform::create(
                NewAccountPlatform {
                    account: account.id,
                    platform: AccountPlatformType::Discord,
                    platform_user: discord_user_id,
                },
                pool,
            )
            .await;
        } else {
            tracing::info!("Account found and matched. Just login");

            // fetch the known account platform tied to this account
            account_platform = database::platform::from_account(&account, AccountPlatformType::Discord, pool).await;
        }

        sync_result.account_token = account.token;
        sync_result.account_token_secret = account.token_secret;
    }

    if let Some(mut account_platform) = account_platform {
        // everytime we log in, we are going to write out this information here
        let discord_user_name = if discord_user.discriminator == "0" {
            discord_user.username.clone()
        } else {
            format!("{}#{}", discord_user.username, discord_user.discriminator)
        };

        let display_name = if let Some(discord_display_name) = discord_user.display_name {
            discord_display_name
        } else if let Some(global) = discord_user.global_name {
            global
        } else {
            discord_user_name.clone()
        };

        let data = vec![
            NewAccountPlatformData {
                key: "discord_id".to_string(),
                value: account_platform.platform_user.clone(),
            },
            NewAccountPlatformData {
                key: "username".to_string(),
                value: discord_user_name.clone(),
            },
            NewAccountPlatformData {
                key: "display_name".to_string(),
                value: display_name.clone(),
            },
            NewAccountPlatformData {
                key: "avatar".to_string(),
                value: discord_user.avatar.unwrap_or_default(),
            },
        ];

        // write the metadata out to be linked to the platform
        database::platform_data::write(&account_platform, &data, pool).await;
        database::platform::update(&mut account_platform, pool).await;

        sync_result.display_name = display_name;
        sync_result.username = discord_user_name;

        Some(sync_result)
    } else {
        None
    }
}

use crate::{
    database::account::AccountLinkedPlatformsResult, routes::profile::ProfileView, sync::discord::MemberSyncResult,
};
use levelcrush::{
    alias::UnixTimestamp,
    anyhow,
    app::{process::ApplicationProcess, settings::ApplicationSettings, Application, ApplicationState},
    cache::MemoryCache,
    database,
    entities::application_user_settings,
    env::EnvVar,
    reqwest,
    retry_lock::RetryLock,
    task_pool::TaskPool,
    tokio::sync::RwLock,
    tracing,
    uuid::Uuid,
};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct AccountExtension {
    pub http_client: reqwest::Client,
    pub profiles: MemoryCache<ProfileView>,
    pub mass_searches: MemoryCache<Vec<AccountLinkedPlatformsResult>>,
    pub searches: MemoryCache<AccountLinkedPlatformsResult>,
    pub challenges: MemoryCache<ProfileView>,
    pub link_gens: MemoryCache<MemberSyncResult>,
    pub guard: RetryLock,
    pub allowed_discords: Vec<String>,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_validate_url: String,
    pub discord_bot_token: String,
    pub bungie_client_id: String,
    pub bungie_client_secret: String,
    pub bungie_api_key: String,
    pub twitch_client_id: String,
    pub twitch_client_secret: String,
    pub twitch_validate_url: String,
    pub server_port: u16,
    pub server_secret: String,
    pub server_host: String,
    pub fallback_url: String,
    pub account_key: String,
}

impl AccountExtension {
    /// Construct an app state
    ///
    /// Note: This will create a new database pool as well as a new bungie client
    pub fn new() -> AccountExtension {
        let http_client = reqwest::ClientBuilder::new()
            .build()
            .expect("Failed to initialize TLS or get system configuration");

        AccountExtension {
            http_client,
            ..Default::default()
        }
    }

    /// setup an entire application environment, using the environment variables
    pub async fn app_stack(
        db_core_connections: u32,
        db_app_connections: u32,
        process_name: &str,
    ) -> anyhow::Result<(
        Application<AccountExtension>,
        ApplicationState<AccountExtension>,
        ApplicationSettings<AccountExtension>,
        ApplicationProcess<AccountExtension>,
    )> {
        tracing::info!("Setting up datbase connections");
        let db_core_url = levelcrush::env::get(EnvVar::DatabaseUrlCore);
        let db_core = levelcrush::database::connect(&db_core_url, db_core_connections).await;

        let db_app_url = levelcrush::env::get(EnvVar::DatabaseUrlSelf);
        let db = levelcrush::database::connect(&db_app_url, db_app_connections).await;

        tracing::info!("Setting up state");
        let mut app_state = ApplicationState {
            database: db,
            database_core: db_core,
            tasks: TaskPool::new(10),
            locks: RetryLock::default(),
            extension: AccountExtension::new(),
        };
        let mut app = Application::env(&app_state).await?;

        let global_process = app.process(process_name).await?;
        global_process.log_info("Loading application settings").await;

        let mut app_settings = ApplicationSettings::load(&app).await?;

        // make sure all settings are populated
        let server_secret = app_settings.get_global("server.secret").unwrap_or_default();
        let server_port = app_settings
            .get_global("server.port")
            .unwrap_or_default()
            .parse::<u16>()
            .unwrap_or(3001);

        let discord_client_id = app_settings.get_global("discord.client_id").unwrap_or_default();
        let discord_client_secret = app_settings.get_global("discord.client_secret").unwrap_or_default();
        let discord_oauth_validate = app_settings.get_global("discord.validate_url").unwrap_or_default();
        let allowed_discords = app_settings.get_global("discord.server_list").unwrap_or_default();

        let fallback_url = app_settings.get_global("server.fallback_url").unwrap_or_default();

        let bungie_id = app_settings.get_global("bungie.client_id").unwrap_or_default();
        let bungie_client_secret = app_settings.get_global("bungie.client_secret").unwrap_or_default();

        let bungie_api_key = app_settings.get_global("bungie.api_key").unwrap_or_default();

        let twitch_client_id = app_settings.get_global("twitch.client_id").unwrap_or_default();
        let twitch_client_secret = app_settings.get_global("twitch.client_secret").unwrap_or_default();
        let twitch_validate_url = app_settings.get_global("twitch.validate_url").unwrap_or_default();

        let server_host = app_settings.get_global("server.host").unwrap_or_default();

        let account_key = app_settings.get_global("account.key").unwrap_or_default();
        // save settings back in. This makes sure they exist

        let sp_setting = server_port.to_string();
        let handles = vec![
            app_settings.set_global("server.port", &sp_setting).await?,
            app_settings.set_global("server.secret", &server_secret).await?,
            app_settings.set_global("discord.client_id", &discord_client_id).await?,
            app_settings
                .set_global("discord.client_secret", &discord_client_secret)
                .await?,
            app_settings
                .set_global("discord.validate_url", &discord_oauth_validate)
                .await?,
            app_settings.set_global("server.fallback_url", &fallback_url).await?,
            app_settings.set_global("account.key", &account_key).await?,
            app_settings.set_global("server.host", &server_host).await?,
            app_settings.set_global("bungie.client_id", &bungie_id).await?,
            app_settings
                .set_global("bungie.client_secret", &bungie_client_secret)
                .await?,
            app_settings.set_global("bungie.api_key", &bungie_api_key).await?,
            app_settings.set_global("twitch.client_id", &twitch_client_id).await?,
            app_settings
                .set_global("twitch.client_secret", &twitch_client_secret)
                .await?,
            app_settings
                .set_global("twitch.validate_url", &twitch_validate_url)
                .await?,
            app_settings
                .set_global("discord.server_list", &allowed_discords)
                .await?,
        ];

        // set inside the extension
        app_state.extension.discord_client_id = discord_client_id;
        app_state.extension.discord_client_secret = discord_client_secret;
        app_state.extension.discord_validate_url = discord_oauth_validate;
        app_state.extension.server_port = server_port;
        app_state.extension.server_secret = server_secret;
        app_state.extension.fallback_url = fallback_url;
        app_state.extension.account_key = account_key;
        app_state.extension.server_host = server_host;
        app_state.extension.bungie_client_id = bungie_id;
        app_state.extension.bungie_client_secret = bungie_client_secret;
        app_state.extension.bungie_api_key = bungie_api_key;
        app_state.extension.twitch_client_id = twitch_client_id;
        app_state.extension.twitch_client_secret = twitch_client_secret;
        app_state.extension.twitch_validate_url = twitch_validate_url;
        app_state.extension.allowed_discords = allowed_discords
            .split(',')
            .map(|v| v.to_string())
            .collect::<Vec<String>>();

        // wait on all handles to finish
        levelcrush::futures::future::join_all(handles).await;

        Ok((app, app_state, app_settings, global_process))
    }
}

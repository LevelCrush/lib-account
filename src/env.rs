/// Set of values that can be found in the .env file
pub enum AppVariable {
    // server variables
    ServerUrl,
    ServerPort,
    ServerSecret,
    ServerFallbackUrl,

    // database url
    DatabaseUrl,

    // http client settings
    DangerAppInvalidCerts,

    // host variables
    HostMain,
    HostAccounts,
    HostAssets,

    // discord
    DiscordValidateUrl,
    DiscordClientId,
    DiscordClientSecret,
    DiscordPublicKey,
    DiscordBotToken,

    // twitch
    TwitchClientId,
    TwitchClientSecret,
    TwitchValidateUrl,

    // bungie conf
    BungieClientId,
    BungieClientSecret,
    BungieApiKey,
    BungieValidateUrl,

    AccountKey,
}

impl From<AppVariable> for &'static str {
    fn from(app_var: AppVariable) -> Self {
        match app_var {
            AppVariable::ServerUrl => "SERVER_URL",
            AppVariable::ServerPort => "SERVER_PORT",
            AppVariable::ServerSecret => "SERVER_SECRET",
            AppVariable::DatabaseUrl => "DATABASE_URL",
            AppVariable::DangerAppInvalidCerts => "DANGER_APP_INVALID_CERTS",
            AppVariable::HostMain => "HOST_MAIN",
            AppVariable::HostAssets => "HOST_ASSETS",
            AppVariable::DiscordClientId => "DISCORD_CLIENT_ID",
            AppVariable::DiscordClientSecret => "DISCORD_CLIENT_SECRET",
            AppVariable::DiscordPublicKey => "DISCORD_PUBLIC_KEY",
            AppVariable::DiscordValidateUrl => "DISCORD_VALIDATE_URL",
            AppVariable::TwitchClientId => "TWITCH_CLIENT_ID",
            AppVariable::TwitchClientSecret => "TWITCH_CLIENT_SECRET",
            AppVariable::TwitchValidateUrl => "TWITCH_VALIDATE_URL",
            AppVariable::BungieClientId => "BUNGIE_CLIENT_ID",
            AppVariable::BungieClientSecret => "BUNGIE_CLIENT_SECRET",
            AppVariable::BungieApiKey => "BUNGIE_API_KEY",
            AppVariable::BungieValidateUrl => "BUNGIE_VALIDATE_URL",
            AppVariable::ServerFallbackUrl => "SERVER_FALLBACK_URL",
            AppVariable::HostAccounts => "HOST_ACCOUNTS",
            AppVariable::DiscordBotToken => "DISCORD_BOT_TOKEN",
            AppVariable::AccountKey => "ACCOUNT_KEY",
        }
    }
}

/// fetches a application variable from the .env file or targeted system environment variables
pub fn get(app_var: AppVariable) -> String {
    std::env::var::<&'static str>(app_var.into()).unwrap_or_default()
}

pub fn exists(app_var: AppVariable) -> bool {
    std::env::var::<&'static str>(app_var.into()).is_ok()
}

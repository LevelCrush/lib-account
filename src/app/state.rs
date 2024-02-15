use crate::{
    database::account::AccountLinkedPlatformsResult, routes::profile::ProfileView,
    sync::discord::MemberSyncResult,
};
use levelcrush::{
    alias::UnixTimestamp, cache::MemoryCache, database, reqwest, retry_lock::RetryLock, tracing,
    uuid::Uuid,
};
use sqlx::SqlitePool;
#[derive(Clone, Debug)]
pub struct AppState {
    pub database: SqlitePool,
    pub http_client: reqwest::Client,
    pub profiles: MemoryCache<ProfileView>,
    pub mass_searches: MemoryCache<Vec<AccountLinkedPlatformsResult>>,
    pub searches: MemoryCache<AccountLinkedPlatformsResult>,
    pub challenges: MemoryCache<ProfileView>,
    pub link_gens: MemoryCache<MemberSyncResult>,
    pub guard: RetryLock,
}

impl AppState {
    /// Construct an app state
    ///
    /// Note: This will create a new database pool as well as a new bungie client
    pub async fn new() -> AppState {
        let max_connections = std::env::var("DATABASE_CONNECTIONS_MAX")
            .unwrap_or_default()
            .parse::<u32>()
            .unwrap_or(1);

        tracing::info!("Connecting to database with {max_connections} connection(s)");
        let database = database::connect(crate::database::DATABASE_URL, max_connections).await;
        tracing::info!("Connection to database established");

        let http_client = reqwest::ClientBuilder::new()
            .build()
            .expect("Failed to initialize TLS or get system configuration");

        AppState {
            database,
            http_client,
            profiles: MemoryCache::new(),
            mass_searches: MemoryCache::new(),
            searches: MemoryCache::new(),
            guard: RetryLock::default(),
            challenges: MemoryCache::new(),
            link_gens: MemoryCache::new(),
        }
    }
}

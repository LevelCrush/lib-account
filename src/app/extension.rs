use crate::{
    database::account::AccountLinkedPlatformsResult, routes::profile::ProfileView,
    sync::discord::MemberSyncResult,
};
use levelcrush::{
    alias::UnixTimestamp, cache::MemoryCache, database, reqwest, retry_lock::RetryLock, tracing,
    uuid::Uuid,
};

#[derive(Clone, Debug)]
pub struct AccountExtension {
    pub http_client: reqwest::Client,
    pub profiles: MemoryCache<ProfileView>,
    pub mass_searches: MemoryCache<Vec<AccountLinkedPlatformsResult>>,
    pub searches: MemoryCache<AccountLinkedPlatformsResult>,
    pub challenges: MemoryCache<ProfileView>,
    pub link_gens: MemoryCache<MemberSyncResult>,
    pub guard: RetryLock,
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
            profiles: MemoryCache::new(),
            mass_searches: MemoryCache::new(),
            searches: MemoryCache::new(),
            guard: RetryLock::default(),
            challenges: MemoryCache::new(),
            link_gens: MemoryCache::new(),
        }
    }
}

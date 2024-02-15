use crate::app::session::SessionKey;
use crate::app::state::AppState;
use crate::{app, database};
use axum::extract::State;
use axum::Router;
use axum::{routing::get, routing::post, Json};
use axum_sessions::extractors::ReadableSession;
use levelcrush::cache::{CacheDuration, CacheValue};
use levelcrush::server::APIResponse;
use levelcrush::util::unix_timestamp;
use levelcrush::uuid::Uuid;
use levelcrush::{axum, tracing};
use levelcrush::{axum_sessions, md5};
use std::collections::HashMap;

pub const CACHE_KEY_PROFILE: &str = "profile||";

#[derive(serde::Serialize, Default, Clone, Debug)]
pub struct ProfileView {
    pub display_name: String,
    pub platforms: HashMap<String, HashMap<String, String>>,
    pub is_admin: bool,
    pub challenge: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct ChallengePayload {
    pub challenge: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(json_view))
        .route("/json", get(json_view))
        .route("/challenge", post(challenge_view))
}

pub async fn challenge_view(
    State(state): State<AppState>,
    Json(payload): Json<ChallengePayload>,
) -> Json<APIResponse<ProfileView>> {
    let mut response = APIResponse::new();

    let challenge_profile = state.challenges.access(&payload.challenge).await;
    if challenge_profile.is_some() {
        tracing::info!("Found challenge match!: {}", payload.challenge);
    }
    response.data(challenge_profile);

    response.complete();
    Json(response)
}

fn generate_challenge(display_name: &str, admin: i64) -> String {
    let uuid = Uuid::new_v4().to_string();
    let challenge_digest = md5::compute(format!(
        "{}{}{}{}",
        unix_timestamp(),
        display_name,
        admin,
        uuid
    ));
    format!("{:x}", challenge_digest)
}

/// output a json view of the data related to the currently logged in session
pub async fn json_view(
    State(mut state): State<AppState>,
    session: ReadableSession,
) -> Json<APIResponse<ProfileView>> {
    let mut response = APIResponse::new();
    let session_id = session.id();
    let cache_key = format!("{}{}", CACHE_KEY_PROFILE, session_id);

    // load session and fetch any relevant information
    let account_token =
        app::session::read::<String>(SessionKey::Account, &session).unwrap_or_default();
    let account_token_secret =
        app::session::read::<String>(SessionKey::AccountSecret, &session).unwrap_or_default();

    let mut profile_view = None;
    let mut account = None;
    let display_name =
        app::session::read::<String>(SessionKey::DisplayName, &session).unwrap_or_default();
    if !account_token.is_empty() && !account_token_secret.is_empty() {
        tracing::info!(
            "Checking if profile is being fetched already for: {}",
            display_name
        );

        // this will cover any request that come in **after** the first one
        let retries = state.guard.wait_until_release(&cache_key).await;
        if retries > 0 {
            tracing::info!(
                "Took {} attempts to release guard for {}",
                retries,
                display_name
            );
        }
        profile_view = state.profiles.access(&cache_key).await
    }

    if profile_view.is_some() {
        tracing::info!("Returning from cache: {}", display_name);
    }

    let mut fetch_profile =
        profile_view.is_none() && !account_token.is_empty() && !account_token_secret.is_empty();

    if fetch_profile {
        tracing::info!("Locking  profile request for {}", display_name);
        let retries = state.guard.lock(&cache_key).await;
        if retries > 0 {
            // we had to wait! this means we may be able to pull from our cache
            profile_view = state.profiles.access(&cache_key).await;
            fetch_profile = profile_view.is_none();
            if profile_view.is_some() {
                // unlock the guard in the case that we do have profile data cached
                state.guard.unlock(&cache_key).await;
            }
        }
    }

    if fetch_profile {
        tracing::info!("Fetching info from db!: {}", account_token);
        account = database::account::get(
            account_token.as_str(),
            account_token_secret,
            &state.database,
        )
        .await;
    }

    if fetch_profile {
        if let Some(account) = account {
            // fetch account related data
            let mut display_name = String::new();

            tracing::info!("Fetching platforms from db!: {}", account_token);
            let platforms = database::account::all_data(&account, &state.database).await;

            // loop through the platform data nad find a platform that is discord related and pull information from there
            for (platform, platform_data) in platforms.iter() {
                if platform.contains("discord") {
                    display_name = match platform_data.get("display_name") {
                        Some(dn) => dn.clone(),
                        _ => String::new(),
                    };
                    break; // no need to continue, it is only possible for our account to have one discord linked account at a time
                }
            }

            // we will only keep this profile in the challenge cache for 5 minutes
            tracing::info!("Storing in challenge cache!: {}", display_name);
            let challenge_hash = generate_challenge(&display_name, account.admin);
            let data = ProfileView {
                display_name,
                platforms,
                is_admin: account.admin == 1,
                challenge: challenge_hash.clone(),
            };

            // intentionally keep this challenge cache around longer then the profile cache result
            state
                .challenges
                .write(
                    challenge_hash,
                    CacheValue::with_duration(
                        data.clone(),
                        CacheDuration::TenMinutes,
                        CacheDuration::TenMinutes,
                    ),
                )
                .await;

            // save into cache
            tracing::info!("Storing in cache!: {}", data.display_name);
            state
                .profiles
                .write(
                    cache_key.clone(),
                    CacheValue::with_duration(
                        data.clone(),
                        CacheDuration::Minute,
                        CacheDuration::FiveMinutes,
                    ),
                )
                .await;

            profile_view = Some(data);
        } else {
            response.error("user", "User not found");
        }

        // make sure we have unlocked the guard.
        tracing::info!("Unlocking profile request: {}", display_name);
        state.guard.unlock(&cache_key).await;
    }

    response.data(profile_view);
    response.complete();

    // return response
    Json(response)
}

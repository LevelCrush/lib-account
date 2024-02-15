use crate::app::state::AppState;
use crate::database;
use crate::database::account::AccountLinkedPlatformsResult;
use axum::extract::State;
use axum::Router;
use axum::{routing::get, Json};
use levelcrush::axum::extract::Path;
use levelcrush::axum::routing::post;
use levelcrush::cache::{CacheDuration, CacheValue};
use levelcrush::tracing;
use levelcrush::{axum, server::APIResponse};
use std::collections::HashMap;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/by/discord/:discord", get(discord_search))
        .route("/by/bungie/:bungie", get(bungie_search))
        .route("/by/bungie", post(bungie_search_mass))
}

async fn discord_search(
    State(mut state): State<AppState>,
    Path(discord): Path<String>,
) -> Json<APIResponse<AccountLinkedPlatformsResult>> {
    let mut response = APIResponse::new();

    let cache_key = format!("search_discord||{}", discord);

    let linked_accounts = if let Some(data) = state.searches.access(&cache_key).await {
        Some(data)
    } else {
        let results = database::account::by_discord(discord, &state.database).await;
        if let Some(results) = &results {
            state
                .searches
                .write(
                    &cache_key,
                    CacheValue::with_duration(results.clone(), CacheDuration::Minute, CacheDuration::Minute),
                )
                .await;
        }
        results
    };

    if linked_accounts.is_some() {
        response.data(linked_accounts);
    } else {
        response.error("bungie", "Could not find a match");
    }

    response.complete();
    Json(response)
}

pub async fn bungie_search(
    State(mut state): State<AppState>,
    Path(bungie): Path<String>,
) -> Json<APIResponse<AccountLinkedPlatformsResult>> {
    let mut response = APIResponse::new();

    let cache_key = format!("search_bungie||{}", bungie);

    let linked_accounts = if let Some(data) = state.searches.access(&cache_key).await {
        Some(data)
    } else {
        let results = database::account::by_bungie(bungie, &state.database).await;
        if let Some(results) = &results {
            state
                .searches
                .write(
                    &cache_key,
                    CacheValue::with_duration(results.clone(), CacheDuration::Minute, CacheDuration::Minute),
                )
                .await;
        }
        results
    };

    if linked_accounts.is_some() {
        response.data(linked_accounts);
    } else {
        response.error("bungie", "Could not find a match");
    }

    response.complete();
    Json(response)
}

pub async fn bungie_search_mass(
    State(mut state): State<AppState>,
    payload: Option<Json<Vec<String>>>,
) -> Json<APIResponse<HashMap<String, Option<AccountLinkedPlatformsResult>>>> {
    let mut response = APIResponse::new();

    if let Some(requested_names) = payload {
        let requested_names = requested_names.iter().cloned().collect::<Vec<String>>();

        let flat_names = requested_names.join(",");
        let cache_key = format!("mass_search_bungie||{}", flat_names);

        let cached_results = state.mass_searches.access(&cache_key).await;
        let mut update_cache = false;
        let linked_accounts = if let Some(data) = cached_results {
            tracing::info!("Returning cache for mass search");
            update_cache = false;
            data
        } else {
            tracing::info!("Fetching mass search info");
            update_cache = true;
            database::account::by_bungie_bulk(&requested_names, &state.database).await
        };

        if update_cache {
            tracing::info!("Caching mass search info");
            state
                .mass_searches
                .write(
                    &cache_key,
                    CacheValue::with_duration(linked_accounts.clone(), CacheDuration::Minute, CacheDuration::Minute),
                )
                .await;
        }

        let mut linked_account_map = linked_accounts
            .into_iter()
            .map(|result| (result.bungie.clone(), Some(result)))
            .collect::<HashMap<String, Option<AccountLinkedPlatformsResult>>>();

        // now backfill any missing entries with None
        for name in requested_names.into_iter() {
            linked_account_map.entry(name).or_insert(None);
        }

        response.data(Some(linked_account_map));
    }

    response.complete();
    Json(response)
}

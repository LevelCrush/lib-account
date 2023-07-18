use levelcrush::{
    axum::{
        extract::{Path, Query, State},
        routing::get,
        Json, Router,
    },
    server::APIResponse,
    tracing,
    util::unix_timestamp,
};
use ts_rs::TS;

use crate::{app::state::AppState, database};

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../lib-levelcrush-ts/src/service-discord/")]
pub struct CategoryActiveUser {
    pub member_id: String,
    pub message_timestamp: String,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../lib-levelcrush-ts/src/service-discord/")]
pub struct ChannelActiveUser {
    pub member_id: String,
    pub message_timestamp: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TimestampQuery {
    pub timestamp: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/guilds/:guild/categories/:category/users/active",
            get(active_category_users),
        )
        .route(
            "/guilds/:guild/channels/:channel/users/active",
            get(active_channel_users),
        )
}

pub async fn active_channel_users(
    Path((guild, channel)): Path<(String, String)>,
    Query(options): Query<TimestampQuery>,
    State(state): State<AppState>,
) -> Json<APIResponse<Vec<CategoryActiveUser>>> {
    let mut response = APIResponse::new();
    let timestamp = options.timestamp.unwrap_or_default();
    let timestamp = timestamp.parse::<u64>().unwrap_or(unix_timestamp() - 3600);
    tracing::info!("Timestamp {} | Channel {}", timestamp, channel);

    let results = database::channel::active_users(&guild, &channel, timestamp, &state.database).await;

    let data = results
        .into_iter()
        .map(|r| CategoryActiveUser {
            member_id: r.member_id.to_string(),
            message_timestamp: r.message_timestamp.to_string(),
        })
        .collect::<Vec<CategoryActiveUser>>();

    response.data(Some(data));

    response.complete();
    Json(response)
}

pub async fn active_category_users(
    Path((guild, category)): Path<(String, String)>,
    Query(options): Query<TimestampQuery>,
    State(state): State<AppState>,
) -> Json<APIResponse<Vec<CategoryActiveUser>>> {
    let mut response = APIResponse::new();
    let timestamp = options.timestamp.unwrap_or_default();
    let timestamp = timestamp.parse::<u64>().unwrap_or(unix_timestamp() - 3600);
    tracing::info!("Timestamp {} | Category {}", timestamp, category);

    let results = database::category::active_users(&guild, &category, timestamp, &state.database).await;

    let data = results
        .into_iter()
        .map(|r| CategoryActiveUser {
            member_id: r.member_id.to_string(),
            message_timestamp: r.message_timestamp.to_string(),
        })
        .collect::<Vec<CategoryActiveUser>>();

    response.data(Some(data));

    response.complete();
    Json(response)
}
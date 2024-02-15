use crate::app::state::AppState;
use axum::Router;
use levelcrush::axum;

pub mod bungie;
pub mod discord;
pub mod twitch;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct OAuthLoginQueries {
    pub redirect: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct OAuthLoginValidationQueries {
    pub error: Option<String>,
    pub code: Option<String>,
    pub state: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct OAuthLoginValidationRequest {
    pub client_id: String,
    pub client_secret: String,
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub scope: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/discord", discord::router())
        .nest("/twitch", twitch::router())
        .nest("/bungie", bungie::router())
}

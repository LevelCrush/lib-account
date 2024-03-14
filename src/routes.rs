pub mod guards;
pub mod link;
pub mod platform;
pub mod profile;
pub mod responses;
pub mod search;
use crate::app::extension::AccountExtension;
use crate::routes::platform::OAuthLoginQueries;
use axum::extract::Query;
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use levelcrush::app::ApplicationState;
use levelcrush::axum::extract::State;
use levelcrush::axum_sessions::extractors::WritableSession;
use levelcrush::tracing;
use levelcrush::{axum, urlencoding};

pub fn router() -> Router<ApplicationState<AccountExtension>> {
    Router::new()
        .route("/login", get(login))
        .route("/logout", get(logout))
        .nest("/platform", platform::router())
        .nest("/profile", profile::router())
        .nest("/search", search::router())
        .nest("/link", link::router())
}

pub async fn login(
    State(state): State<ApplicationState<AccountExtension>>,
    Query(login_fields): Query<OAuthLoginQueries>,
) -> Redirect {
    // make sure we know where to return our user to after they are done logging in
    let final_fallback_url = state.extension.fallback_url;
    let final_redirect = login_fields.redirect.unwrap_or(final_fallback_url);

    let path = format!(
        "/platform/discord/login?redirect={}",
        urlencoding::encode(final_redirect.as_str())
    );

    tracing::info!("Redirect path!: {}", path);
    Redirect::temporary(path.as_str())
}

pub async fn logout(
    State(state): State<ApplicationState<AccountExtension>>,
    Query(login_fields): Query<OAuthLoginQueries>,
    mut session: WritableSession,
) -> Redirect {
    let final_fallback_url = state.extension.fallback_url;
    let final_redirect = login_fields.redirect.unwrap_or(final_fallback_url);

    // destroy session
    session.destroy();

    tracing::info!("Redirect path!: {}", &final_redirect);
    Redirect::temporary(&final_redirect)
}

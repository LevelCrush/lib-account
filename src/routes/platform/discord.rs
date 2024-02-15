use crate::app::session::SessionKey;
use crate::app::state::AppState;
use crate::env::AppVariable;
use crate::routes::platform::{OAuthLoginQueries, OAuthLoginValidationQueries};
use crate::{app, env};
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use axum_sessions::extractors::WritableSession;
use levelcrush::tracing;
use levelcrush::util::unix_timestamp;
use levelcrush::{axum, urlencoding};
use levelcrush::{axum_sessions, md5};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login))
        .route("/validate", get(validate))
}

pub async fn login(
    Query(login_fields): Query<OAuthLoginQueries>,
    mut session: WritableSession,
) -> Redirect {
    // make sure we know where to return our user to after they are done logging in
    let server_url = env::get(AppVariable::ServerUrl);
    let fallback_url = env::get(AppVariable::ServerFallbackUrl);
    let final_fallback_url = fallback_url;
    let final_redirect = login_fields.redirect.unwrap_or(final_fallback_url);

    let client_id = env::get(AppVariable::DiscordClientId);
    let authorize_redirect = env::get(AppVariable::DiscordValidateUrl);
    let scopes = vec!["identify"].join("+");

    let hash_input = md5::compute(format!("{}||{}", client_id, unix_timestamp()));
    let discord_state = format!("{:x}", hash_input);
    let authorize_url = format!("https://discord.com/api/oauth2/authorize?response_type={}&client_id={}&scope={}&state={}&redirect_uri={}&prompt={}",
                                "code",
                                urlencoding::encode(client_id.as_str()),
                                scopes,
                                urlencoding::encode(discord_state.as_str()),
                                urlencoding::encode(authorize_redirect.as_str()),
                                "none"//"consent"
    );

    // store discord state check and final redirect in session
    app::session::write(
        SessionKey::PlatformDiscordState,
        discord_state,
        &mut session,
    );

    // store original url that this route was called from
    app::session::write(
        SessionKey::PlatformDiscordCallerUrl,
        final_redirect,
        &mut session,
    );

    // Now redirect
    Redirect::temporary(authorize_url.as_str())
}

pub async fn validate(
    Query(validation_query): Query<OAuthLoginValidationQueries>,
    State(mut state): State<AppState>,
    mut session: WritableSession,
) -> Redirect {
    let query_fields = validation_query;
    // make sure we know where to return our user to after they are done logging in
    let fallback_url = env::get(AppVariable::ServerFallbackUrl);
    let final_fallback_url = fallback_url;

    let final_redirect = app::session::read(SessionKey::PlatformDiscordCallerUrl, &session)
        .unwrap_or(final_fallback_url);

    let mut do_process = true;
    let validation_state = query_fields.state.unwrap_or_default();
    let session_state = app::session::read::<String>(SessionKey::PlatformDiscordState, &session)
        .unwrap_or_default();

    let oauth_code = query_fields.code.unwrap_or_default();
    let oauth_error = query_fields.error.unwrap_or_default();

    // make sure we don't have an error and we have a code that we can check
    if !oauth_error.is_empty() {
        do_process = false;
        tracing::warn!(
            "There was an error found in the oauth request {}",
            oauth_error
        );
    }

    if oauth_code.is_empty() {
        tracing::warn!("There was no code present in the oauth request");
        do_process = false;
    }

    if validation_state != session_state {
        tracing::warn!(
            "Validation State and Session state did not match: Discord ({}) || Session({})",
            validation_state,
            session_state
        );
        do_process = false;
    }

    // if we are not yet allowed to process then go ahead and simply return immediately to our final redirect url that we know about
    if !do_process {
        return Redirect::temporary(final_redirect.as_str());
    }

    // now validate the code returned to us if we are allowed to process
    let validation_response = app::discord::validate_oauth(&oauth_code, &state).await;
    let member_sync = if let Some(validation) = validation_response {
        app::discord::member_oauth(&validation.access_token, &state).await
    } else {
        None
    };

    if let Some(member) = member_sync {
        app::session::login(&mut session, member);
    }

    let discord_username =
        app::session::read::<String>(SessionKey::Username, &session).unwrap_or_default();
    let search_cache_key = format!("search_discord||{}", discord_username);
    tracing::info!("Busting search key: {}", search_cache_key);
    state.searches.delete(&search_cache_key).await;

    // no matter what we redirect back to our caller
    Redirect::temporary(final_redirect.as_str())
}

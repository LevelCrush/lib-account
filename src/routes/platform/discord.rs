use crate::app;
use crate::app::extension::AccountExtension;
use crate::app::session::SessionKey;
use crate::routes::platform::{OAuthLoginQueries, OAuthLoginValidationQueries};
use crate::routes::responses::DiscordGuild;
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use axum_sessions::extractors::WritableSession;
use levelcrush::app::ApplicationState;
use levelcrush::tracing;
use levelcrush::util::unix_timestamp;
use levelcrush::{axum, urlencoding};
use levelcrush::{axum_sessions, md5};

pub fn router() -> Router<ApplicationState<AccountExtension>> {
    Router::new()
        .route("/login", get(login))
        .route("/validate", get(validate))
}

pub async fn login(
    State(state): State<ApplicationState<AccountExtension>>,
    Query(login_fields): Query<OAuthLoginQueries>,
    mut session: WritableSession,
) -> Redirect {
    // make sure we know where to return our user to after they are done logging in
    let server_url = state.extension.server_host.clone();
    let fallback_url = state.extension.fallback_url.clone();
    let final_fallback_url = fallback_url;
    let final_redirect = login_fields.redirect.unwrap_or(final_fallback_url);

    let client_id = state.extension.discord_client_id;
    let authorize_redirect = state.extension.discord_validate_url;
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
    app::session::write(SessionKey::PlatformDiscordState, discord_state, &mut session);

    // store original url that this route was called from
    app::session::write(SessionKey::PlatformDiscordCallerUrl, final_redirect, &mut session);

    // Now redirect
    Redirect::temporary(authorize_url.as_str())
}

pub async fn validate(
    Query(validation_query): Query<OAuthLoginValidationQueries>,
    State(mut state): State<ApplicationState<AccountExtension>>,
    mut session: WritableSession,
) -> Redirect {
    let query_fields = validation_query;
    // make sure we know where to return our user to after they are done logging in
    let fallback_url = state.extension.fallback_url.clone();
    let final_fallback_url = fallback_url;

    let mut final_redirect =
        app::session::read(SessionKey::PlatformDiscordCallerUrl, &session).unwrap_or(final_fallback_url);

    let mut do_process = true;
    let validation_state = query_fields.state.unwrap_or_default();
    let session_state = app::session::read::<String>(SessionKey::PlatformDiscordState, &session).unwrap_or_default();

    let oauth_code = query_fields.code.unwrap_or_default();
    let oauth_error = query_fields.error.unwrap_or_default();

    // make sure we don't have an error and we have a code that we can check
    if !oauth_error.is_empty() {
        do_process = false;
        tracing::warn!("There was an error found in the oauth request {}", oauth_error);
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
    let mut access_token = String::new();
    let member_sync = if let Some(validation) = validation_response {
        access_token = validation.access_token.to_string();
        app::discord::member_oauth(&access_token, &state).await
    } else {
        None
    };

    // now get the list of guilds they are in and if they have at least one of the guild ids in the allowed server list then we are in good shape
    let is_allowed = if let Some(member) = &member_sync {
        let discord_guilds = app::discord::member_oauth_guilds_api(&access_token, &state).await;
        discord_guilds
            .into_iter()
            .any(|g| state.extension.allowed_discords.contains(&g.id))
    } else {
        false
    };

    if is_allowed {
        if let Some(member) = member_sync {
            app::session::login(&mut session, member);
        }

        let discord_username = app::session::read::<String>(SessionKey::Username, &session).unwrap_or_default();
        let search_cache_key = format!("search_discord||{}", discord_username);
        tracing::info!("Busting search key: {}", search_cache_key);
        state.extension.searches.delete(&search_cache_key).await;
    } else {
        let param_type = if final_redirect.contains("?") { "&" } else { "?" };
        final_redirect = format!("{final_redirect}{param_type}error=NotAllowed")
    }

    // no matter what we redirect back to our caller
    Redirect::temporary(final_redirect.as_str())
}

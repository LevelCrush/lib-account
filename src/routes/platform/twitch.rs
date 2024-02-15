use crate::app::session::SessionKey;
use crate::app::state::AppState;
use crate::database::platform::{AccountPlatformType, NewAccountPlatform};
use crate::database::platform_data::NewAccountPlatformData;
use crate::env::AppVariable;
use crate::routes::guards;
use crate::routes::platform::{
    OAuthLoginQueries, OAuthLoginValidationQueries, OAuthLoginValidationRequest,
};
use crate::routes::profile::CACHE_KEY_PROFILE;
use crate::{app, database, env};
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use axum_sessions::extractors::{ReadableSession, WritableSession};
use levelcrush::axum_sessions;
use levelcrush::tracing;
use levelcrush::util::unix_timestamp;
use levelcrush::{axum, md5, urlencoding};

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct TwitchValidationResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct TwitchUserResponse {
    pub data: Vec<TwitchUserData>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct TwitchUserData {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub profile_image_url: String,
    pub offline_image_url: String,
    pub description: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login))
        .route("/validate", get(validate))
        .route("/unlink", get(unlink))
        .route_layer(axum::middleware::from_fn(guards::session_logged_in))
}

pub async fn unlink(
    Query(fields): Query<OAuthLoginQueries>,
    State(mut state): State<AppState>,
    session: ReadableSession,
) -> Redirect {
    //extract query fields
    let query_fields = fields;
    let session_id = session.id();

    let cache_key = format!("{}{}", CACHE_KEY_PROFILE, session_id);

    // make sure we know where to return our user to after they are done logging in
    let server_url = env::get(AppVariable::ServerUrl);
    let fallback_url = env::get(AppVariable::ServerFallbackUrl);

    let final_fallback_url = fallback_url;
    let final_redirect = query_fields.redirect.unwrap_or(final_fallback_url);

    // get account tied to session
    let session_account_token =
        app::session::read::<String>(SessionKey::Account, &session).unwrap_or_default();
    let session_account_secret =
        app::session::read::<String>(SessionKey::AccountSecret, &session).unwrap_or_default();

    // look up account in the database
    let account = database::account::get(
        session_account_token,
        session_account_secret,
        &state.database,
    )
    .await;

    // find the account platform tied to this account.
    let mut account_platform = None;
    if account.is_some() {
        let account = account.unwrap();
        account_platform = database::platform::from_account(
            &account,
            AccountPlatformType::Twitch,
            &state.database,
        )
        .await;
    }

    // if we found it , we can go ahead and perform all of our unlink operations on it
    if account_platform.is_some() {
        let account_platform = account_platform.unwrap();
        database::platform::unlink(&account_platform, &state.database).await;
    }

    tracing::info!("Unlinking!");
    tracing::info!("Busting cache on profile at {}", cache_key);
    state.profiles.delete(&cache_key).await;

    let discord_username =
        app::session::read::<String>(SessionKey::Username, &session).unwrap_or_default();
    let search_cache_key = format!("search_discord||{}", discord_username);
    tracing::info!("Busting search key: {}", search_cache_key);
    state.searches.delete(&search_cache_key).await;

    // Now redirect
    Redirect::temporary(final_redirect.as_str())
}

pub async fn login(
    Query(login_fields): Query<OAuthLoginQueries>,
    mut session: WritableSession,
) -> Redirect {
    let query_fields = login_fields;

    // make sure we know where to return our user to after they are done logging in
    let server_url = env::get(AppVariable::ServerUrl);
    let fallback = env::get(AppVariable::ServerFallbackUrl);
    let final_fallback_url = fallback;
    let final_redirect = query_fields.redirect.unwrap_or(final_fallback_url);

    let client_id = env::get(AppVariable::TwitchClientId);
    let authorize_redirect = env::get(AppVariable::TwitchValidateUrl);
    let scopes = vec!["user:read:email"].join("+");

    let hash_input = md5::compute(format!("{}||{}", client_id, unix_timestamp()));
    let twitch_state = format!("{:x}", hash_input);
    let authorize_url = format!("https://id.twitch.tv/oauth2/authorize?response_type={}&client_id={}&scope={}&state={}&redirect_uri={}&force_verify={}",
                                "code",
                                urlencoding::encode(client_id.as_str()),
                                scopes,
                                urlencoding::encode(twitch_state.as_str()),
                                urlencoding::encode(authorize_redirect.as_str()),
                                "false"//"consent"
    );

    // store discord state check and final redirect in session
    app::session::write(SessionKey::PlatformTwitchState, twitch_state, &mut session);

    // store original url that this route was called from
    app::session::write(
        SessionKey::PlatformTwitchCallerUrl,
        final_redirect,
        &mut session,
    );

    // Now redirect
    Redirect::temporary(authorize_url.as_str())
}

pub async fn validate(
    Query(validation_query): Query<OAuthLoginValidationQueries>,
    State(mut state): State<AppState>,
    session: ReadableSession,
) -> Redirect {
    let query_fields = validation_query;

    let session_id = session.id();
    let cache_key = format!("{}{}", CACHE_KEY_PROFILE, session_id);

    let server_url = env::get(AppVariable::ServerUrl);
    let fallback_url = env::get(AppVariable::ServerFallbackUrl);
    let final_fallback_url = fallback_url;
    let final_redirect =
        app::session::read::<String>(SessionKey::PlatformTwitchCallerUrl, &session)
            .unwrap_or(final_fallback_url);

    let mut do_process = true;
    let validation_state = query_fields.state.unwrap_or_default();
    let session_state =
        app::session::read::<String>(SessionKey::PlatformTwitchState, &session).unwrap_or_default();

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
    let mut validation_response = None;

    if do_process {
        let client_id = env::get(AppVariable::TwitchClientId);
        let client_secret = env::get(AppVariable::TwitchClientSecret);
        let authorize_redirect = env::get(AppVariable::TwitchValidateUrl);
        let scopes = vec!["user:read:email"].join("+");

        let request = state
            .http_client
            .post("https://id.twitch.tv/oauth2/token")
            .body(
                serde_urlencoded::to_string(OAuthLoginValidationRequest {
                    client_id: client_id.clone(),
                    client_secret: client_secret.clone(),
                    grant_type: "authorization_code".to_string(),
                    code: oauth_code.clone(),
                    redirect_uri: authorize_redirect.clone(),
                    scope: scopes,
                })
                .unwrap_or_default(),
            )
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .send()
            .await;

        if request.is_ok() {
            let oauth_result = request.unwrap().json::<TwitchValidationResponse>().await;
            if oauth_result.is_ok() {
                validation_response = oauth_result.ok();
            } else {
                tracing::error!("Could not parse validation response for twitch!");
                validation_response = None;
            }
        }
    }

    do_process = validation_response.is_some();
    let mut twitch_user = None;
    let mut access_token = String::new();
    let mut refresh_token = String::new();
    if do_process {
        let validation_response = validation_response.unwrap_or_default();
        access_token = validation_response.access_token.clone();
        refresh_token = validation_response.refresh_token.clone();

        let request = state
            .http_client
            .get("https://api.twitch.tv/helix/users")
            .bearer_auth(access_token.clone())
            .header("Client-Id", env::get(AppVariable::TwitchClientId))
            .header("Accept", "application/json")
            .send()
            .await;

        if request.is_ok() {
            let request = request.unwrap();
            let result = request.json::<TwitchUserResponse>().await;
            if result.is_ok() {
                twitch_user = result.ok();
            } else {
                let error = result.err().unwrap();
                tracing::error!("{}", error);
                twitch_user = None;
            }
        } else {
            tracing::error!("Could not parse twitch user response");
            twitch_user = None;
        }
    }

    do_process = twitch_user.is_some();

    // extract out the twitch user data if possible ahead of time
    let twitch_user = if twitch_user.is_some() {
        let twitch_user_response = twitch_user.unwrap_or_default();
        let twitch_default_user_data = TwitchUserData::default();
        let twitch_user_data = twitch_user_response
            .data
            .get(0)
            .unwrap_or(&twitch_default_user_data);
        twitch_user_data.clone()
    } else {
        TwitchUserData::default()
    };

    // only run this block if we have some twitch user data present in our response
    // no point in querying the database if we have no way to link it
    let mut account = None;
    if do_process {
        let session_account_token =
            app::session::read::<String>(SessionKey::Account, &session).unwrap_or_default();
        let session_account_secret =
            app::session::read::<String>(SessionKey::AccountSecret, &session).unwrap_or_default();

        // look up account in the database
        account = database::account::get(
            session_account_token,
            session_account_secret,
            &state.database,
        )
        .await;
    }

    // allow processing if we have a linked account from our session information
    do_process = account.is_some();

    // unwrap the session account from our option and provide a default version
    // note: this default version wont be used if we do not allow processing like we check on above so its just a safety mechanism
    let account = account.unwrap_or_default();
    let mut account_platform = None;
    if do_process {
        tracing::info!("Matching Twitch Account");
        account_platform = database::platform::read(
            AccountPlatformType::Twitch,
            twitch_user.id.clone(),
            &state.database,
        )
        .await;
    }

    // we can process this block so long as we have a valid account to work with
    // platform not needed since we can insert a new platform if necessary
    let new_platform = account_platform.is_none();
    if do_process && new_platform {
        tracing::info!("New twitch account needs to be linked");
        account_platform = database::platform::create(
            NewAccountPlatform {
                account: account.id,
                platform: AccountPlatformType::Twitch,
                platform_user: twitch_user.id.clone(),
            },
            &state.database,
        )
        .await;
    } else if do_process && !new_platform {
        let mut account_platform_record = account_platform.unwrap_or_default();
        tracing::info!("Twitch account can be updated to link to current session account");

        account_platform_record.account = account.id;

        // update the platform data
        account_platform =
            database::platform::update(&mut account_platform_record, &state.database).await;
    }

    // if we have linked our account submit it to the metadata section of our database to update
    do_process = account_platform.is_some();
    if do_process {
        // insert/update from our discord user response to update things like display name/etc
        let account_platform = account_platform
            .expect("No account platform was found. even though it should of been there");

        let data = vec![
            NewAccountPlatformData {
                key: "twitch_id".to_string(),
                value: twitch_user.id.to_string(),
            },
            NewAccountPlatformData {
                key: "display_name".to_string(),
                value: twitch_user.display_name,
            },
            NewAccountPlatformData {
                key: "offline_image_url".to_string(),
                value: twitch_user.offline_image_url,
            },
            NewAccountPlatformData {
                key: "profile_image_url".to_string(),
                value: twitch_user.profile_image_url,
            },
            NewAccountPlatformData {
                key: "login".to_string(),
                value: twitch_user.login,
            },
            NewAccountPlatformData {
                key: "description".to_string(),
                value: twitch_user.description,
            },
        ];

        // update profile metadata
        database::platform_data::write(&account_platform, &data, &state.database).await;
    }

    // bust cache key
    tracing::info!("Busting cache key: {}", cache_key);
    state.profiles.delete(&cache_key).await;

    let discord_username =
        app::session::read::<String>(SessionKey::Username, &session).unwrap_or_default();
    let search_cache_key = format!("search_discord||{}", discord_username);
    tracing::info!("Busting search key: {}", search_cache_key);
    state.searches.delete(&search_cache_key).await;

    // no matter what we redirect back to our caller
    Redirect::temporary(final_redirect.as_str())
}

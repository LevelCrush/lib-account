use crate::app::session::SessionKey;
use crate::app::state::AppState;
use crate::database::platform::{AccountPlatformType, NewAccountPlatform};
use crate::database::platform_data::NewAccountPlatformData;
use crate::env::AppVariable;
use crate::routes::guards;
use crate::routes::platform::{OAuthLoginQueries, OAuthLoginValidationQueries};
use crate::routes::profile::CACHE_KEY_PROFILE;
use crate::{app, database, env};
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use axum_sessions::extractors::{ReadableSession, WritableSession};
use levelcrush::tracing;
use levelcrush::util::unix_timestamp;
use levelcrush::{axum, reqwest};
use levelcrush::{axum_sessions, urlencoding};
use levelcrush::{md5, tokio};
use reqwest::header::HeaderMap;
use tokio::join;

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct BungieValidationResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub membership_id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct BungieUserData {
    #[serde(rename = "membershipId")]
    pub membership_id: String,

    #[serde(rename = "uniqueName")]
    pub unique_name: String,

    #[serde(rename = "displayName")]
    pub display_name: String,

    #[serde(default, rename = "xboxDisplayName")]
    pub xbox_display_name: String,

    #[serde(default, rename = "blizzardDisplayName")]
    pub blizzard_display_name: String,

    #[serde(default, rename = "steamDisplayName")]
    pub steam_display_name: String,

    #[serde(default, rename = "twitchDisplayName")]
    pub twitch_display_name: String,

    #[serde(default, rename = "stadiaDisplayName")]
    pub stadia_display_name: String,

    #[serde(default, rename = "egsDisplayName")]
    pub egs_display_name: String,

    #[serde(default, rename = "psnDisplayName")]
    pub psn_display_name: String,

    #[serde(default, rename = "cachedBungieGlobalDisplayName")]
    pub global_display_name: String,

    #[serde(default, rename = "cachedBungieGlobalDisplayNameCode")]
    pub global_display_name_code: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct BungieMembership {
    #[serde(rename = "LastSeenDisplayName")]
    pub last_seen_display_name: String,

    #[serde(rename = "applicableMembershipTypes")]
    pub applicable_membership_types: Vec<i32>,

    #[serde(rename = "membershipId")]
    pub membership_id: String,

    #[serde(rename = "membershipType")]
    pub membership_type: i32,

    #[serde(rename = "displayName")]
    pub display_name: String,

    #[serde(rename = "bungieGlobalDisplayName")]
    pub global_display_name: String,

    #[serde(rename = "bungieGlobalDisplayNameCode")]
    pub global_display_name_code: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct BungieMembershipData {
    #[serde(rename = "destinyMemberships")]
    pub memberships: Vec<BungieMembership>,

    #[serde(rename = "primaryMembershipId")]
    pub primary_membership_id: String,

    #[serde(rename = "bungieNetUser")]
    pub net_user: BungieUserData,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct BungieResponse<T> {
    #[serde(rename = "Response")]
    pub response: T,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct OAuthLoginValidationRequest {
    pub grant_type: String,
    pub code: String,
}

/// this is setup by comparing bungie net platform membershiop types
/// and comparing to raid report platforms
/// https://bungie-net.github.io/#/components/schemas/BungieMembershipType
fn get_membership_name(membership_type: i32) -> &'static str {
    match membership_type {
        0 => "none", // 0 is verified to be none
        1 => "xb",   // 1 is verified to be xbox
        2 => "ps",   // 2 is verified to be playstation
        _ => "pc", // other numbers either result in epic game store, steam, battle.net, or unknown numbers. Values like -1 are not possible and value 254 is reserved and not used
    }
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
    let final_fallback_url = env::get(AppVariable::ServerFallbackUrl);
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
            AccountPlatformType::Bungie,
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
    let final_fallback_url = env::get(AppVariable::ServerFallbackUrl);
    let final_redirect = query_fields.redirect.unwrap_or(final_fallback_url);

    let client_id = env::get(AppVariable::BungieClientId);
    let hash_input = md5::compute(format!("{}||{}", client_id, unix_timestamp()));
    let bungie_state = format!("{:x}", hash_input);
    let authorize_url = format!(
        "https://www.bungie.net/en/OAuth/Authorize?response_type={}&client_id={}&state={}&prompt={}",
        "code",
        urlencoding::encode(client_id.as_str()),
        urlencoding::encode(bungie_state.as_str()),
        "prompt"
    );

    // store discord state check and final redirect in session
    app::session::write(SessionKey::PlatformBungieState, bungie_state, &mut session);

    // store original url that this route was called from
    app::session::write(
        SessionKey::PlatformBungieCallerUrl,
        final_redirect,
        &mut session,
    );

    // Now redirect
    Redirect::temporary(authorize_url.as_str())
}

pub async fn validate(
    headers: HeaderMap,
    Query(validation_query): Query<OAuthLoginValidationQueries>,
    State(mut state): State<AppState>,
    session: ReadableSession,
) -> Redirect {
    let query_fields = validation_query;

    let session_id = session.id();
    let cache_key = format!("{}{}", CACHE_KEY_PROFILE, session_id);
    let final_fallback_url = env::get(AppVariable::ServerFallbackUrl);
    let final_redirect =
        app::session::read::<String>(SessionKey::PlatformBungieCallerUrl, &session)
            .unwrap_or(final_fallback_url);

    let mut do_process = true;
    let validation_state = query_fields.state.unwrap_or_default();
    let session_state =
        app::session::read::<String>(SessionKey::PlatformBungieState, &session).unwrap_or_default();

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
            "Validation State and Session state did not match: Bungie ({}) || Session({})",
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
    tracing::info!("Validating Bungie OAUTH");
    let mut validation_response: Option<BungieValidationResponse> = None;
    let api_key = env::get(AppVariable::BungieApiKey);
    if do_process {
        let client_id = env::get(AppVariable::BungieClientId);
        let client_secret = env::get(AppVariable::BungieClientSecret);
        let form_body = serde_urlencoded::to_string(OAuthLoginValidationRequest {
            grant_type: "authorization_code".to_string(),
            code: oauth_code.clone(),
        })
        .unwrap_or_default();

        let request = state
            .http_client
            .post("https://www.bungie.net/Platform/App/OAuth/token/")
            .body(form_body)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .header("X-API-KEY", api_key.clone())
            .basic_auth(client_id, Some(client_secret))
            .send()
            .await;

        if request.is_ok() {
            let oauth_result = request.unwrap();
            let oauth_result = oauth_result.json::<BungieValidationResponse>().await;

            if oauth_result.is_ok() {
                validation_response = oauth_result.ok();
            } else {
                tracing::error!("Could not parse validation response for bungie!");
                let err = oauth_result.err().unwrap();
                tracing::error!("{}", err);
                validation_response = None;
            }
        } else {
            let err = request.err().unwrap();
            tracing::error!("Request Error: {}", err);
        }
    }

    do_process = validation_response.is_some();

    let mut access_token = String::new();
    let mut membership_id = String::new();
    let mut membership_data = None;
    let mut user_data = None;
    if do_process {
        tracing::info!("Validated. Getting more information about the user");
        let validation_response = validation_response.unwrap_or_default();
        access_token = validation_response.access_token.clone();
        membership_id = validation_response.membership_id.clone();

        // construct our endpoint urls that we will need to run
        let bungie_user_endpoint = format!(
            "https://www.bungie.net/Platform/User/GetBungieNetUserById/{}/",
            membership_id
        );
        let bungie_membership_endpoint = format!(
            "https://www.bungie.net/Platform/User/GetMembershipsById/{}/-1/",
            membership_id
        );

        let user_request_future = state
            .http_client
            .get(bungie_user_endpoint)
            .bearer_auth(access_token.as_str())
            .header("X-API-KEY", api_key.as_str())
            .header("Accept", "application/json")
            .send();

        let membership_request_future = state
            .http_client
            .get(bungie_membership_endpoint)
            .bearer_auth(access_token.clone())
            .header("X-API-KEY", api_key.as_str())
            .header("Accept", "application/json")
            .send();

        let (user_response, membership_response) =
            join!(user_request_future, membership_request_future);

        if user_response.is_ok() {
            user_data = Some(
                user_response
                    .unwrap()
                    .json::<BungieResponse<BungieUserData>>()
                    .await
                    .unwrap_or_default(),
            );
        } else {
            user_data = None;
        }

        if membership_response.is_ok() {
            membership_data = Some(
                membership_response
                    .unwrap()
                    .json::<BungieResponse<BungieMembershipData>>()
                    .await
                    .unwrap_or_default(),
            );
        } else {
            membership_data = None;
        }
    }

    // so long as we have made a call to both the user endpoint and membership endpoints we can continue on here
    do_process = user_data.is_some() && membership_data.is_some();

    // extract our user data if possible or use defaults
    let user_data = user_data.unwrap_or_default().response;
    let membership_data = membership_data.unwrap_or_default().response;

    // fetch account from session
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

    // we do indeed have an account tied to our session so we can continue on
    do_process = account.is_some();

    // unwrap the session account from our option and provide a default version
    // note: this default version wont be used if we do not allow processing like we check on above so its just a safety mechanism
    let account = account.unwrap_or_default();
    let mut account_platform = None;
    if do_process {
        tracing::info!("Matching Bungie Account");
        account_platform = database::platform::read(
            AccountPlatformType::Bungie,
            user_data.membership_id.clone(),
            &state.database,
        )
        .await;
    }

    // we can process this block so long as we have a valid account to work with
    // platform not needed since we can insert a new platform if necessary
    let new_platform = account_platform.is_none();
    if do_process && new_platform {
        tracing::info!("New bungie account needs to be linked");
        account_platform = database::platform::create(
            NewAccountPlatform {
                account: account.id,
                platform: AccountPlatformType::Bungie,
                platform_user: user_data.membership_id.clone(),
            },
            &state.database,
        )
        .await;
    } else if do_process && !new_platform {
        let mut account_platform_record = account_platform.unwrap_or_default();
        tracing::info!("Bungie account can be updated to link to current account session");
        account_platform_record.account = account.id;

        // update the platform data
        account_platform =
            database::platform::update(&mut account_platform_record, &state.database).await;
    }

    do_process = account_platform.is_some();
    if do_process {
        let account_platform = account_platform
            .expect("No account platform was found. even though there should be something here");
        let mut data = vec![
            NewAccountPlatformData {
                key: "bungie_id".to_string(),
                value: user_data.membership_id.to_string(),
            },
            NewAccountPlatformData {
                key: "primary_membership_id".to_string(),
                value: membership_data.primary_membership_id.clone(),
            },
            NewAccountPlatformData {
                key: "display_name".to_string(),
                value: user_data.display_name,
            },
            NewAccountPlatformData {
                key: "unique_name".to_string(),
                value: user_data.unique_name,
            },
        ];

        let mut primary_platform_type = 0;
        let mut primary_platform_name = "";
        let mut membership_types = Vec::new();
        // now loop through memberships and add some information about them as well into our metadata
        for membership in membership_data.memberships.iter() {
            // perform a check to see if this is the primary membership , this will only ever trigger once
            let is_primary_membership =
                membership_data.primary_membership_id == membership.membership_id;
            if is_primary_membership {
                primary_platform_type = membership.membership_type;
                primary_platform_name = get_membership_name(primary_platform_type);

                data.push(NewAccountPlatformData {
                    key: "primary_platform".to_string(),
                    value: primary_platform_type.to_string(),
                });

                data.push(NewAccountPlatformData {
                    key: "primary_platform_abbr".to_string(),
                    value: primary_platform_name.to_string(),
                })
            }

            let membership_key = format!("membership_{}", membership.membership_type);
            let membership_id_key = format!("{}_id", membership_key);
            let membership_display_name_key = format!("{}_display_name", membership_key);

            // store membership id
            data.push(NewAccountPlatformData {
                key: membership_id_key,
                value: membership.membership_id.clone(),
            });

            // store membership display name
            data.push(NewAccountPlatformData {
                key: membership_display_name_key,
                value: membership.display_name.clone(),
            });

            membership_types.push(membership.membership_type.to_string());
        }

        // also store as a comma seperated list the membership types we have tied to this account platform
        data.push(NewAccountPlatformData {
            key: "memberships".to_string(),
            value: membership_types.join(","),
        });

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

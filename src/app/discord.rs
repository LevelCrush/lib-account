use super::extension::AccountExtension;
use crate::routes::responses::DiscordUserGuildsResponse;
use crate::{
    routes::{
        platform::OAuthLoginValidationRequest,
        responses::{DiscordUserResponse, DiscordValidationResponse},
    },
    sync,
    sync::discord::MemberSyncResult,
};
use levelcrush::{app::ApplicationState, axum::headers::AccessControlRequestMethod, env, tracing};

pub async fn validate_oauth(
    oauth_code: &str,
    state: &ApplicationState<AccountExtension>,
) -> Option<DiscordValidationResponse> {
    let client_id = state.extension.discord_client_id.clone();
    let client_secret = state.extension.discord_client_secret.clone();
    let authorize_redirect = state.extension.discord_validate_url.clone();
    let scopes = vec!["identify"].join("+");

    let request = state
        .extension
        .http_client
        .post("https://discord.com/api/oauth2/token")
        .body(
            serde_urlencoded::to_string(OAuthLoginValidationRequest {
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
                grant_type: "authorization_code".to_string(),
                code: oauth_code.to_string(),
                redirect_uri: authorize_redirect.clone(),
                scope: scopes,
            })
            .unwrap_or_default(),
        )
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .send()
        .await;

    if let Ok(response) = request {
        let json = response.json::<DiscordValidationResponse>().await;
        if let Ok(data) = json {
            Some(data)
        } else {
            let err = json.err().unwrap();
            tracing::error!("Could not parse oauth validation response! {}", err);
            None
        }
    } else {
        None
    }
}

/// queries a discord user directly by their discord id
pub async fn member_api(discord_id: &str, state: &ApplicationState<AccountExtension>) -> Option<DiscordUserResponse> {
    let bot_token = state.extension.discord_bot_token.clone();
    let bot_auth = format!("Bot {}", bot_token);
    let discord_user_id = discord_id;

    let endpoint = format!("https://discord.com/api/v10/users/{}", discord_user_id);
    let request = state
        .extension
        .http_client
        .get(&endpoint)
        .header("Authorization", bot_auth)
        .send()
        .await;

    if let Ok(request) = request {
        let json = request.json::<DiscordUserResponse>().await;
        if let Ok(json) = json {
            Some(json)
        } else {
            let err = json.err().unwrap();
            tracing::error!("Error occurred while parsing user response:\r\n{}", err);
            None
        }
    } else {
        None
    }
}

/// fetches a discord member by their id and syncs the result
pub async fn member(discord_id: &str, state: &ApplicationState<AccountExtension>) -> Option<MemberSyncResult> {
    let discord_response = member_api(discord_id, state).await;
    if let Some(user_response) = discord_response {
        sync::discord::member(user_response, &state).await
    } else {
        None
    }
}

/// query discord api with oauth authentication
pub async fn member_oauth_api(
    access_token: &str,
    state: &ApplicationState<AccountExtension>,
) -> Option<DiscordUserResponse> {
    let request = state
        .extension
        .http_client
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(access_token)
        .send()
        .await;

    if let Ok(result) = request {
        let json = result.json::<DiscordUserResponse>().await;
        if let Ok(data) = json {
            Some(data)
        } else {
            let err = json.err().unwrap();
            tracing::error!("Unable to parse oauth validation response: {}", err);
            None
        }
    } else {
        None
    }
}

/// Query the discord api directly and get the currently logged in users guild list
pub async fn member_oauth_guilds_api(
    access_token: &str,
    state: &ApplicationState<AccountExtension>,
) -> Option<DiscordUserGuildsResponse> {
    let request = state
        .extension
        .http_client
        .get("https://discord.com/api/v10/users/@me/guilds")
        .bearer_auth(access_token)
        .send()
        .await;

    if let Ok(response) = request {
        let json = response.json::<DiscordUserGuildsResponse>().await;
        if let Ok(data) = json {
            Some(data)
        } else {
            tracing::error!("Failed to parse incoming json for User Guild request");
            None
        }
    } else {
        None
    }
}

/// Query a member via oauth authentication
/// Update them in our database
pub async fn member_oauth(access_token: &str, state: &ApplicationState<AccountExtension>) -> Option<MemberSyncResult> {
    let oauth_response = member_oauth_api(access_token, state).await;
    if let Some(user_response) = oauth_response {
        sync::discord::member(user_response, &state).await
    } else {
        None
    }
}

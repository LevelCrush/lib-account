use super::responses::LinkGeneratedResponse;
use crate::{
    app::{self, state::AppState},
    env::{self, AppVariable},
};
use axum::Router;
use levelcrush::{
    axum::{
        self,
        extract::{Path, Query, State},
        http::HeaderMap,
        response::Redirect,
        routing::{get, post},
        Json,
    },
    axum_sessions::extractors::WritableSession,
    cache::{CacheDuration, CacheValue},
    md5,
    server::APIResponse,
    urlencoding,
    util::{slugify, unix_timestamp},
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, Debug)]
pub struct LinkGeneratePayload {
    pub id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, Debug)]
pub struct LinkQuery {
    pub code: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/generate", post(link_generate))
        .route("/platform/:platform", get(link_platform))
        .route("/done", get(link_done))
        .route("/bad", get(link_bad))
}

async fn link_generate(
    headers: HeaderMap,
    State(mut state): State<AppState>,
    Json(payload): Json<LinkGeneratePayload>,
) -> Json<APIResponse<LinkGeneratedResponse>> {
    let key_header = match headers.get("Account-Key") {
        Some(header_value) => header_value
            .to_str()
            .expect("Unable to convert header value to str"),
        _ => "",
    };

    let server_key = env::get(AppVariable::AccountKey);
    if server_key != key_header {
        return Json(APIResponse::new());
    }

    let mut response = APIResponse::new();

    let member = app::discord::member(&payload.id, &state).await;
    if let Some(member) = member {
        let input: String = format!(
            "{}@{}::{}@{}",
            member.account_token_secret,
            member.username,
            unix_timestamp(),
            member.account_token,
        );
        let md5_digest = md5::compute(input);
        let hash = format!("{:x}", md5_digest);

        // store our hash
        // whena  user makes a request to /link/bungie or /link/twitch with  ?code=hash , if the has is found in link_gen cache, then we will trust them
        // this will only stay in the cache for 5 minutes.
        state
            .link_gens
            .write(
                hash.clone(),
                CacheValue::with_duration(
                    member,
                    CacheDuration::FiveMinutes,
                    CacheDuration::FiveMinutes,
                ),
            )
            .await;

        response.data(Some(LinkGeneratedResponse { code: hash }));
    }

    response.complete();
    Json(response)
}

async fn link_platform(
    Query(query): Query<LinkQuery>,
    Path(target_platform): Path<String>,
    State(mut state): State<AppState>,
    mut session: WritableSession,
) -> Redirect {
    let mut link_code = String::new();
    let member = {
        if let Some(code) = query.code {
            let sync_result = state.link_gens.access(&code).await;
            link_code = code.clone();
            sync_result
        } else {
            None
        }
    };

    if let Some(member) = member {
        app::session::login(&mut session, member);
        let platform = slugify(&target_platform.to_lowercase());
        let done_url = format!(
            "{}/link/done?code={}",
            env::get(AppVariable::HostAccounts),
            urlencoding::encode(&link_code),
        );
        let redirect_url = format!(
            "{}/platform/{}/login?redirect={}",
            env::get(AppVariable::HostAccounts),
            urlencoding::encode(&platform),
            urlencoding::encode(&done_url)
        );
        Redirect::temporary(&redirect_url)
    } else {
        let redirect_url = format!("{}/link/bad", env::get(AppVariable::HostAccounts));
        Redirect::temporary(&redirect_url)
    }
}

async fn link_done(
    Query(query): Query<LinkQuery>,
    State(mut state): State<AppState>,
) -> &'static str {
    if let Some(code) = query.code {
        state.link_gens.delete(&code).await;
    }
    "Thank you for linking your account. You can close this tab/window now"
}

async fn link_bad() -> &'static str {
    "Your code is either expired/incorrect or there was a problem starting the link brocess"
}

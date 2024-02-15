use crate::app;
use crate::sync::discord::MemberSyncResult;
use axum_sessions::async_session::Session;
use levelcrush::axum_sessions;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub enum SessionKey {
    Account,
    AccountSecret,
    DisplayName,
    Username,
    PlatformDiscordCallerUrl,
    PlatformDiscordState,
    PlatformTwitchCallerUrl,
    PlatformTwitchState,
    PlatformBungieCallerUrl,
    PlatformBungieState,
}

impl From<SessionKey> for &'static str {
    fn from(session: SessionKey) -> Self {
        match session {
            SessionKey::Account => "account",
            SessionKey::AccountSecret => "account_secret",
            SessionKey::DisplayName => "display_name",
            SessionKey::Username => "username",
            SessionKey::PlatformDiscordCallerUrl => "platform_discord_caller_url",
            SessionKey::PlatformDiscordState => "platform_discord_state",
            SessionKey::PlatformTwitchCallerUrl => "platform_twitch_caller_url",
            SessionKey::PlatformTwitchState => "platform_twitch_state",
            SessionKey::PlatformBungieCallerUrl => "platform_bungie_caller_url",
            SessionKey::PlatformBungieState => "platform_bungie_state",
            _ => panic!("No match for this session key"),
        }
    }
}

impl From<SessionKey> for String {
    fn from(session: SessionKey) -> Self {
        let rep: &str = session.into();
        rep.to_string()
    }
}

/**
 * Reads a variable from the session store
 */
pub fn read<T: DeserializeOwned>(key: SessionKey, session: &Session) -> Option<T> {
    session.get(key.into())
}

/**
 * Writes into the specified session store
 */
pub fn write<T: Serialize>(key: SessionKey, value: T, session: &mut Session) {
    session.insert(key.into(), value).ok();
}

/** Clear the session of known possible session keys
 */
pub fn clear(session: &mut Session) {
    session.remove(SessionKey::PlatformDiscordCallerUrl.into());
    session.remove(SessionKey::PlatformDiscordState.into());
    session.remove(SessionKey::AccountSecret.into());
    session.remove(SessionKey::Account.into());
    session.remove(SessionKey::DisplayName.into());
    session.remove(SessionKey::PlatformTwitchCallerUrl.into());
    session.remove(SessionKey::PlatformTwitchState.into());
    session.remove(SessionKey::PlatformBungieCallerUrl.into());
    session.remove(SessionKey::PlatformBungieState.into());
}

pub fn login(session: &mut Session, member: MemberSyncResult) {
    // clear the session variables out, this is safe since discord is our primary login
    app::session::clear(session);

    // in the session store important information related to the account, the account token and the token secret
    app::session::write(SessionKey::Account, member.account_token, session);
    app::session::write(SessionKey::AccountSecret, member.account_token_secret, session);
    app::session::write(SessionKey::DisplayName, member.display_name, session);
    app::session::write(SessionKey::Username, member.username, session);
}

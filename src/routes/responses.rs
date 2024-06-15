use levelcrush::macros::ExternalAPIResponse;

#[ExternalAPIResponse]
pub struct DiscordValidationResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[ExternalAPIResponse]
pub struct DiscordUserResponse {
    pub id: Option<String>,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub global_name: Option<String>,
    pub display_name: Option<String>,
}

#[ExternalAPIResponse]
pub struct DiscordRole {
    pub id: Option<String>,
    pub name: String,
}

#[ExternalAPIResponse]
pub struct DiscordGuild {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub owner: bool,
}

pub type DiscordUserGuildsResponse = Vec<DiscordGuild>;

#[derive(serde::Serialize)]
pub struct LinkGeneratedResponse {
    pub code: String,
}

use crate::{
    app::{self, state::AppState},
    database::{self, platform::AccountPlatformType},
};
use levelcrush::{anyhow, tokio, tracing};
use std::time::Duration;

pub async fn run(args: &[String]) -> anyhow::Result<()> {
    //todo!
    let state = AppState::new().await;
    let limit = {
        if !args.is_empty() {
            match args.first() {
                Some(v) => v.parse::<i64>().unwrap_or_default(),
                _ => 1000,
            }
        } else {
            1000
        }
    };

    let need_update = database::platform::need_update(AccountPlatformType::Discord, limit, &state.database).await;
    for discord_id in need_update.into_iter() {
        tracing::info!("Updating member: {}", discord_id);
        app::discord::member(&discord_id, &state).await;

        // intentionally add a delay between each request of 100ms
        // this is a lazy and innaccurate way of making sure we dont exceed our
        // global rate limit of 50 request per second
        // https://discord.com/developers/docs/topics/rate-limits#global-rate-limit
        // eventually we should examine the request for rate limit info
        // for now this works just fine
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

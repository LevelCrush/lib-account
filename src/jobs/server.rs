use crate::{
    app::state::AppState,
    env::{self, AppVariable},
    routes,
};
use levelcrush::{anyhow, server::Server};
use levelcrush::{tokio, tracing};
use std::time::Duration;

pub async fn run() -> anyhow::Result<()> {
    tracing::info!("Setting up http server for account service");
    let server_port = env::get(AppVariable::ServerPort)
        .parse::<u16>()
        .unwrap_or(3000);

    tracing::info!("Establishing applicatiopn state for account service");
    let app_state = AppState::new().await;

    tracing::info!("Setting up cache prune task for account service");
    let mut app_state_bg = app_state.clone();
    let cache_task = tokio::spawn(async move {
        loop {
            app_state_bg.challenges.prune().await;
            app_state_bg.link_gens.prune().await;
            app_state_bg.profiles.prune().await;
            app_state_bg.mass_searches.prune().await;
            app_state_bg.searches.prune().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    tracing::info!("Running server on port {server_port}");

    (_, _) = tokio::join!(
        Server::new(server_port)
            .enable_cors()
            .enable_session()
            .run(routes::router(), app_state.clone()),
        cache_task
    );

    Ok(())
}

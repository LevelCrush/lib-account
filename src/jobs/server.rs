use crate::app::extension::AccountExtension;
use crate::routes;
use levelcrush::app::process::LogLevel;
use levelcrush::app::settings::ApplicationSettings;
use levelcrush::app::{Application, ApplicationState};
use levelcrush::env::EnvVar;
use levelcrush::retry_lock::RetryLock;
use levelcrush::task_pool::TaskPool;
use levelcrush::{anyhow, server::Server};
use levelcrush::{app, tokio, tracing};
use std::time::Duration;

pub async fn run(db_core_connections: u32, db_app_connections: u32) -> anyhow::Result<()> {
    let (mut app, app_state, app_settings, global_process) =
        AccountExtension::app_state(db_core_connections, db_app_connections).await?;

    let server_port = app_state.extension.server_port;
    let server_secret = app_state.extension.server_secret;

    if server_secret.is_empty() {
        panic!("Please set a server secret");
    }

    global_process
        .log_info("Setting up cache prune task for account service")
        .await;

    let mut app_state_bg = app_state.clone();
    let cache_task = tokio::spawn(async move {
        loop {
            app_state_bg.extension.challenges.prune().await;
            app_state_bg.extension.link_gens.prune().await;
            app_state_bg.extension.profiles.prune().await;
            app_state_bg.extension.mass_searches.prune().await;
            app_state_bg.extension.searches.prune().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    let msg = format!("Running server on port {server_port}");
    global_process.log_info(&msg).await;

    (_, _) = tokio::join!(
        Server::new(server_port)
            .enable_cors()
            .enable_session(&server_secret)
            .run(routes::router(), app_state.clone()),
        cache_task
    );

    Ok(())
}

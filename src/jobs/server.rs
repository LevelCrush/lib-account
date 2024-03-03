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
    tracing::info!("Setting up datbase connections");
    let db_core_url = levelcrush::env::get(EnvVar::DatabaseUrlCore);
    let db_core = levelcrush::database::connect(&db_core_url, db_core_connections).await;

    let db_app_url = levelcrush::env::get(EnvVar::DatabaseUrlSelf);
    let db = levelcrush::database::connect(&db_app_url, db_app_connections).await;

    tracing::info!("Setting up state");
    let mut app_state = ApplicationState {
        database: db,
        database_core: db_core,
        tasks: TaskPool::new(10),
        locks: RetryLock::default(),
        extension: AccountExtension::new(),
    };

    tracing::info!("Application setting up");
    let app = Application::env(&app_state).await?;

    let global_process = app.process("server").await?;
    global_process
        .log_info("Loading application settings")
        .await;

    let mut app_settings = ApplicationSettings::load(&app).await?;

    let server_port = app_settings
        .get_global("server.port")
        .unwrap_or_default()
        .parse::<u16>()
        .unwrap_or(3001);

    let server_secret = app_settings.get_global("server.secret").unwrap_or_default();

    // save these two settings into our application settings. This will ensure they are created properly.

    let sp_setting = server_port.to_string();
    let sp_req = app_settings.set_global("server.port", &sp_setting).await?;
    let ss_req = app_settings
        .set_global("server.secret", &server_secret)
        .await?;

    sp_req.await;
    ss_req.await;

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

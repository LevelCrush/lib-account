use crate::app::extension::AccountExtension;
use crate::routes;
use levelcrush::anyhow;
use levelcrush::server::Server;
use levelcrush::tokio;

pub async fn run(db_core_connections: u32, db_app_connections: u32) -> anyhow::Result<()> {
    let (mut app, app_state, app_settings, global_process) =
        AccountExtension::app_stack(db_core_connections, db_app_connections, "account-server")
            .await?;

    let server_port = app_state.extension.server_port;
    let server_secret = app_state.extension.server_secret.clone();

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

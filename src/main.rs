use app::state::AppState;
use clap::Parser;
use database::platform::AccountPlatformType;
use env::AppVariable;
use jobs::server;
use levelcrush::{clap, server::Server, tokio, tracing};
use std::time::Duration;

mod app;
mod database;
mod env;
pub mod jobs;
mod routes;
mod sync;

#[derive(clap::ValueEnum, Clone, Debug)]
enum Job {
    Server,
    DiscordUpdate,
    Purge,
    Reset,
}

#[derive(clap::Parser, Debug)]
struct Args {
    #[arg(help = "The functionality you intend to run")]
    pub job: Job,

    #[arg(help = "Additional arguments to feed to the job")]
    pub args: Vec<String>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing::info!("Merging .env into process enviorment settings");
    levelcrush::env();

    tracing::info!("Parsing command line arguments");
    let args = Args::parse();

    tracing::info!("Running job");
    let result = match args.job {
        Job::Server => server::run().await,
        Job::DiscordUpdate => jobs::discord::run(&args.args).await,
        Job::Purge => jobs::purge::run().await,
        Job::Reset => jobs::reset::run().await,
    };

    if let Err(err) = result {
        tracing::error!("An error was encountered during the job:\r\n{}", err);
    }
}

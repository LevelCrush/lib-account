mod app;
mod bungie;
mod database;
mod env;
mod jobs;
mod routes;
//mod server;
mod sync;

use clap::Parser;

// use the tokio install that we are using with our level crush library
use levelcrush::tokio;

#[derive(clap::ValueEnum, Clone, Debug)]
enum Job {
    Server,
    SyncManifest,
    ClanInfo,
    ClanRoster,
    ClanCrawl,
    ClanNotNetworkCrawl,
    MemberProfile,
    MemberActivity,
    MemberCrawl,
    MemberCrawlDeep,
    NetworkCrawl,
    InstanceCrawl,
    InstanceProfiles,
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
    // setup the levelcrush env
    levelcrush::env();

    // parse command line arguments
    let args = Args::parse();

    match args.job {
        Job::Server => jobs::server::run().await,
        Job::SyncManifest => jobs::manifest::run().await,
        Job::ClanInfo => jobs::clan::info(&args.args).await,
        Job::ClanRoster => jobs::clan::roster(&args.args).await,
        Job::ClanCrawl => jobs::clan::crawl_direct(&args.args).await,
        Job::ClanNotNetworkCrawl => jobs::clan::crawl_non_network().await,
        Job::MemberProfile => jobs::member::profile(&args.args).await,
        Job::MemberActivity => jobs::activity::history(&args.args).await,
        Job::MemberCrawl => jobs::member::crawl_profile(&args.args).await,
        Job::MemberCrawlDeep => jobs::member::crawl_profile_deep(&args.args).await,
        Job::NetworkCrawl => jobs::clan::crawl_network().await,
        Job::InstanceCrawl => jobs::activity::crawl_instances(&args.args).await,
        Job::InstanceProfiles => jobs::activity::instance_member_profiles(&args.args).await,
    };
}
use levelcrush::{anyhow, env::EnvVar};
use migration::{Migrator, MigratorTrait};

pub async fn up_all() -> anyhow::Result<()> {
    // migrrate up
    let db_app_url = levelcrush::env::get(EnvVar::DatabaseUrlSelf);
    let db = levelcrush::database::connect(&db_app_url, 1).await;
    Migrator::up(&db, None).await?;

    Ok(())
}

pub async fn up(amount: u32) -> anyhow::Result<()> {
    // migrrate up
    let db_app_url = levelcrush::env::get(EnvVar::DatabaseUrlSelf);
    let db = levelcrush::database::connect(&db_app_url, 1).await;
    Migrator::up(&db, Some(amount)).await?;

    Ok(())
}

pub async fn down_all() -> anyhow::Result<()> {
    // migrate down
    let db_app_url = levelcrush::env::get(EnvVar::DatabaseUrlSelf);
    let db = levelcrush::database::connect(&db_app_url, 1).await;
    Migrator::down(&db, None).await?;

    Ok(())
}

pub async fn down(amount: u32) -> anyhow::Result<()> {
    let db_app_url = levelcrush::env::get(EnvVar::DatabaseUrlSelf);
    let db = levelcrush::database::connect(&db_app_url, 1).await;
    Migrator::down(&db, Some(amount)).await?;

    Ok(())
}

pub async fn fresh() -> anyhow::Result<()> {
    // fresh
    let db_app_url = levelcrush::env::get(EnvVar::DatabaseUrlSelf);
    let db = levelcrush::database::connect(&db_app_url, 1).await;
    Migrator::fresh(&db).await?;

    Ok(())
}

pub async fn refresh() -> anyhow::Result<()> {
    let db_app_url = levelcrush::env::get(EnvVar::DatabaseUrlSelf);
    let db = levelcrush::database::connect(&db_app_url, 1).await;
    Migrator::refresh(&db).await?;

    Ok(())
}

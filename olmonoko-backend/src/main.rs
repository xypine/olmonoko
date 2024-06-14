mod auth;
mod logic;
mod models;
mod routes;
mod utils;

use dotenvy::dotenv;
use sqlx::sqlite::SqlitePoolOptions;
use thiserror::Error;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    // fmt::format::FmtSpan,
};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DbErr(#[from] sqlx::error::Error),
    #[error("IO error: {0}")]
    IoErr(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    println!("OLMONOKO {}", built_info::PKG_VERSION,);
    let git_hash = built_info::GIT_COMMIT_HASH.unwrap_or("unknown");
    let dirty = built_info::GIT_DIRTY.unwrap_or(false);
    println!(
        "Built from commit: {}{} on {}",
        git_hash,
        if dirty { " (dirty)" } else { "" },
        built_info::BUILT_TIME_UTC
    );
    dotenv().ok();
    tracing_subscriber::fmt()
        // .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .init();
    tracing::info!("Starting up");
    tracing::info!("Running migrations");
    let pool = get_conn().await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    tracing::info!("Migrations complete");

    tracing::info!("Starting scheduler");
    logic::scheduler::init()
        .await
        .expect("Failed to start scheduler");
    tracing::info!("Scheduler started");

    tracing::info!("Starting server");
    routes::run_server(pool).await?;
    Ok(())
}

async fn get_conn() -> Result<sqlx::SqlitePool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    Ok(pool)
}

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

mod auth;
mod db_utils;
mod logic;
mod middleware;
mod routes;

use chrono::Datelike;
use dotenvy::dotenv;
use thiserror::Error;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};

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
    let git_hash = get_source_commit().unwrap_or("unknown".to_string());
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
    let scheduler = logic::scheduler::init()
        .await
        .expect("Failed to start scheduler");
    tracing::info!("Scheduler started");

    tracing::info!("Starting server");
    routes::run_server(pool, scheduler).await?;
    Ok(())
}

use sqlx::postgres::PgPoolOptions;
pub(crate) async fn get_conn() -> Result<sqlx::PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    Ok(pool)
}

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn get_source_commit() -> Option<String> {
    built_info::GIT_COMMIT_HASH
        .map(|s| s.to_string())
        .or(std::env::var("SOURCE_COMMIT").ok())
}

pub fn get_version() -> String {
    fn to_two_digits(n: u32) -> String {
        (if n < 10 {
            format!("0{}", n)
        } else {
            n.to_string()
        })
        .chars()
        .rev()
        .take(2)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
    }
    let built_at = built::util::strptime(crate::built_info::BUILT_TIME_UTC);
    let commit = get_source_commit();
    let commit_short = commit.as_ref().map(|s| s.chars().take(7).collect());
    let version = format!(
        "v{}{}{}-{}",
        to_two_digits(built_at.year() as u32),
        to_two_digits(built_at.month()),
        to_two_digits(built_at.day()),
        commit_short
            .clone()
            .unwrap_or_else(|| "eeeeeee".to_string())
    );
    version
}

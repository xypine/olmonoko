use chrono::{DateTime, Utc};
use tokio_cron_scheduler::JobScheduler;

pub mod models;
pub mod utils;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub type DatabaseConnection = sqlx::PgPool;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildInformation {
    pub package_version: String,
    pub commit: Option<String>,
    pub commit_short: Option<String>,
    pub build_time: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AppState {
    pub site_url: String,
    pub version: String,

    pub build_info: BuildInformation,

    pub conn: DatabaseConnection,
    pub scheduler: JobScheduler,
    pub templates: tera::Tera,
}

pub fn get_site_url() -> String {
    std::env::var("SITE_URL").expect("SITE_URL must be set")
}

pub fn get_source_commit() -> Option<String> {
    built_info::GIT_COMMIT_HASH
        .map(|s| s.to_string())
        .or(std::env::var("SOURCE_COMMIT").ok())
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct NavigationEntry<'a> {
    pub name: &'a str,
    pub path: &'a str,
    pub active: Option<bool>,
    pub position: isize,
}

// Navigation entries must be listed in [most --> least specific] order
// for automatic path highlighting to work
// (starting the list with the index would result in the index always being selected, for example)
// the final list is reversed because of this restriction
pub const APP_NAVIGATION_ENTRIES_ADMIN: [NavigationEntry; 1] = [NavigationEntry {
    name: "Admin",
    path: "/admin",
    active: None,
    position: 10, // Last
}];
pub const APP_NAVIGATION_ENTRIES_LOGGEDIN: [NavigationEntry; 2] = [
    NavigationEntry {
        name: "Profile",
        path: "/me",
        active: None,
        position: 8, // Second Last
    },
    NavigationEntry {
        name: "Local",
        path: "/local",
        active: None,
        position: 3, // Second
    },
];
pub const APP_NAVIGATION_ENTRIES_LOGGEDOUT: [NavigationEntry; 1] = [NavigationEntry {
    name: "Sign in",
    path: "/me",
    active: None,
    position: 8, // Second last
}];
pub const APP_NAVIGATION_ENTRIES_PUBLIC: [NavigationEntry; 2] = [
    NavigationEntry {
        name: "Sources",
        path: "/remote",
        active: None,
        position: 5, // Middle
    },
    NavigationEntry {
        name: "Home",
        path: "/",
        active: None,
        position: 1, // First
    },
];

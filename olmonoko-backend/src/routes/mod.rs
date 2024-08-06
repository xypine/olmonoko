use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpServer};
use api::meta::BuildInformation;
use chrono::Datelike;
use tokio_cron_scheduler::JobScheduler;
use tracing::info;
use tracing_actix_web::TracingLogger;

mod api;
mod ui;

use crate::middleware::autocache_responder;
use crate::middleware::autocacher::PREDICTIVE_CACHE_ENABLED;
use crate::middleware::AutoCacher;
use actix_web_lab::middleware::from_fn;

pub type DatabaseConnection = sqlx::PgPool;

#[derive(Clone)]
pub(crate) struct AppState {
    pub site_url: String,
    pub version: String,

    pub build_info: BuildInformation,

    pub conn: DatabaseConnection,
    pub scheduler: JobScheduler,
    pub templates: tera::Tera,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct NavigationEntry<'a> {
    pub name: &'a str,
    pub path: &'a str,
    pub active: Option<bool>,
}

// Navigation entries must be listed in [most --> least specific] order
// for automatic path highlighting to work
// (starting the list with the index would result in the index always being selected, for example)
// the final list is reversed because of this restriction
pub const APP_NAVIGATION_ENTRIES_ADMIN: [NavigationEntry; 1] = [NavigationEntry {
    name: "Admin",
    path: "/admin",
    active: None,
}];
pub const APP_NAVIGATION_ENTRIES_LOGGEDIN: [NavigationEntry; 2] = [
    NavigationEntry {
        name: "Profile",
        path: "/me",
        active: None,
    },
    NavigationEntry {
        name: "Local",
        path: "/local",
        active: None,
    },
];
pub const APP_NAVIGATION_ENTRIES_LOGGEDOUT: [NavigationEntry; 1] = [NavigationEntry {
    name: "Sign in",
    path: "/me",
    active: None,
}];
pub const APP_NAVIGATION_ENTRIES_PUBLIC: [NavigationEntry; 2] = [
    NavigationEntry {
        name: "Sources",
        path: "/remote",
        active: None,
    },
    NavigationEntry {
        name: "Home",
        path: "/",
        active: None,
    },
];

pub fn get_site_url() -> String {
    std::env::var("SITE_URL").expect("SITE_URL must be set")
}

pub fn get_source_commit() -> Option<String> {
    crate::built_info::GIT_COMMIT_HASH
        .map(|s| s.to_string())
        .or(std::env::var("SOURCE_COMMIT").ok())
}

pub async fn run_server(conn: DatabaseConnection, scheduler: JobScheduler) -> std::io::Result<()> {
    let templates = tera::Tera::new("templates/**/*").unwrap();
    let site_url = get_site_url();
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
    let package_version = crate::built_info::PKG_VERSION.to_string();
    let state = AppState {
        site_url,
        version,

        build_info: BuildInformation {
            package_version,
            commit,
            commit_short,
            build_time: built_at,
        },

        conn,
        scheduler,
        templates,
    };
    if PREDICTIVE_CACHE_ENABLED {
        info!("Predictive Caching enabled")
    }
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        let autocache = AutoCacher;
        App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .wrap(autocache)
            .wrap(from_fn(autocache_responder))
            .app_data(web::Data::new(state.clone()))
            .service(api::routes())
            .service(
                web::scope("/static")
                    .default_service(Files::new("", "static"))
                    .wrap(
                        actix_web::middleware::DefaultHeaders::new()
                            .add(("Cache-Control", "max-age=31536000")),
                    ),
            )
            .service(ui::routes())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

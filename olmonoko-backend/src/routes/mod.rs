use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpServer};
use chrono::{Datelike, Timelike, Utc};
use tracing_actix_web::TracingLogger;

mod api;
mod ui;

pub type DatabaseConnection = sqlx::SqlitePool;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct AppState {
    pub site_url: String,
    pub version: String,
    pub built_at: chrono::DateTime<Utc>,

    pub conn: DatabaseConnection,
    pub templates: tera::Tera,
}

pub fn get_site_url() -> String {
    std::env::var("SITE_URL").expect("SITE_URL must be set")
}

pub async fn run_server(conn: DatabaseConnection) -> std::io::Result<()> {
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
    let version = format!(
        "v{}{}{}{}",
        to_two_digits(built_at.year() as u32),
        to_two_digits(built_at.month()),
        to_two_digits(built_at.day()),
        to_two_digits(built_at.hour())
    );
    let state = AppState {
        site_url,
        built_at,
        version,

        conn,
        templates,
    };
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(state.clone()))
            .service(api::routes())
            .service(
                web::scope("/static")
                    .default_service(Files::new("", "static"))
                    .wrap(
                        actix_web::middleware::DefaultHeaders::new()
                            .add(("Cache-Control", "max-age=3600")),
                    ),
            )
            .service(ui::routes())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

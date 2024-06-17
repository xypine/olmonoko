use actix_web::{get, web, HttpResponse, Responder, Scope};
use chrono::{DateTime, Utc};

use crate::routes::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildInformation {
    pub package_version: String,
    pub commit: Option<String>,
    pub commit_short: Option<String>,
    pub build_time: DateTime<Utc>,
}

#[get("/version")]
async fn version(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(data.build_info.clone())
}

#[get("/ready")]
async fn ready() -> impl Responder {
    // TODO: Return 503 if not ready
    HttpResponse::Ok().body("OK")
}

// TODO: Add /license, /health, etc

pub fn routes() -> Scope {
    web::scope("/meta").service(version).service(ready)
}

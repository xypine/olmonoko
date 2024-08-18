use actix_web::{get, web, HttpResponse, Responder, Scope};

use olmonoko_common::AppState;

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

use actix_web::{get, web, HttpResponse, Responder, Scope};

#[get("/version")]
async fn version() -> impl Responder {
    HttpResponse::Ok().body("0.1.0")
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

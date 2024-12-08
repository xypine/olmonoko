use actix_web::{get, web, HttpRequest, HttpResponse, Responder, Scope};
use olmonoko_common::AppState;
use serde::Deserialize;

use crate::db_utils::request::{deauth, EnhancedRequest};


#[derive(Debug, Deserialize)]
pub struct NLCEPRequest {
    nl: String
}
#[get("/nlcep")]
async fn nlcep_route(data: web::Data<AppState>, query: web::Query<NLCEPRequest>, req: HttpRequest) -> impl Responder {
    let user = req.get_session_user(&data).await;
    if user.is_some() {
        let query = query.into_inner();
        return match query.nl.parse::<nlcep::NewEvent>() {
            Ok(event) => HttpResponse::Ok().json(event),
            Err(err) => HttpResponse::BadRequest().json(err),
        };
    }
    deauth(&req)
}

pub fn routes() -> Scope {
    web::scope("/ui_utils")
        .service(nlcep_route)
}

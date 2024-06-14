use actix_web::{web, Scope};

pub(crate) mod data_source;
pub(crate) mod event;
pub(crate) mod export;
pub(crate) mod meta;
pub(crate) mod user;

pub fn routes() -> Scope {
    web::scope("/api")
        .service(data_source::routes())
        .service(meta::routes())
        .service(user::routes())
        .service(export::routes())
        .service(event::routes())
}

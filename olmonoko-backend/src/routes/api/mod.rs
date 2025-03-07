use actix_web::{web, Scope};

pub(crate) mod backup;
pub(crate) mod data_source;
pub(crate) mod event;
pub(crate) mod export;
pub(crate) mod key;
pub(crate) mod meta;
pub(crate) mod user;
pub(crate) mod timer;
pub(crate) mod ui_utils;

pub fn routes() -> Scope {
    web::scope("/api")
        .service(data_source::routes())
        .service(meta::routes())
        .service(user::routes())
        .service(export::routes())
        .service(event::routes())
        .service(backup::routes())
        .service(key::routes())
        .service(timer::routes())
        .service(ui_utils::routes())
}

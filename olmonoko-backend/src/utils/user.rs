use actix_web::web;

use crate::{
    models::{
        public_link::{PublicLink, RawPublicLink},
        user::UserId,
    },
    routes::AppState,
};

use super::request::{InternalServerError, OrInternalServerError};

pub async fn get_user_export_links(
    data: &web::Data<AppState>,
    user_id: UserId,
) -> Result<Vec<PublicLink>, InternalServerError<sqlx::Error>> {
    sqlx::query_as!(
        RawPublicLink,
        "SELECT * FROM public_calendar_links WHERE user_id = $1",
        user_id
    )
    .fetch_all(&data.conn)
    .await
    .or_internal_server_error("Failed to query user export links from db")
    .map(|links| links.into_iter().map(PublicLink::from).collect())
}

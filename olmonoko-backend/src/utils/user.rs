use actix_web::web;

use crate::{
    models::public_link::{PublicLink, RawPublicLink},
    routes::AppState,
};

pub async fn get_user_export_links(data: &web::Data<AppState>, user_id: i64) -> Vec<PublicLink> {
    sqlx::query_as!(
        RawPublicLink,
        "SELECT * FROM public_calendar_links WHERE user_id = $1",
        user_id
    )
    .fetch_all(&data.conn)
    .await
    .expect("Failed to query export links from the database")
    .into_iter()
    .map(PublicLink::from)
    .collect()
}

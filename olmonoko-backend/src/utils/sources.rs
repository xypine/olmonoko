use actix_web::web;

use crate::{
    models::ics_source::{IcsSource, RawIcsSource},
    routes::AppState,
};

pub async fn get_visible_sources(
    data: &web::Data<AppState>,
    user_id: Option<i64>,
) -> Vec<IcsSource> {
    sqlx::query!(
        "SELECT s.*, p.priority FROM ics_sources AS s LEFT JOIN ics_source_priorities AS p ON p.ics_source_id = s.id AND p.user_id = $1 WHERE s.is_public = true OR s.user_id = $1",
        user_id
    )
    .fetch_all(&data.conn)
    .await
    .unwrap()
    .into_iter()
    .map(|source| {
        IcsSource::from((RawIcsSource {
            id: source.id,
            name: source.name,
            url: source.url,
            user_id: source.user_id,
            last_fetched_at: source.last_fetched_at,
            is_public: source.is_public,
            created_at: source.created_at,
            persist_events: source.persist_events,
            all_as_allday: source.all_as_allday,
            import_template: source.import_template,
        }, source.priority))
    })
    .collect()
}
pub async fn get_visible_sources_with_event_count(
    data: &web::Data<AppState>,
    user_id: Option<i64>,
) -> Vec<(IcsSource, i64, i64)> {
    sqlx::query!(
        "SELECT COUNT(DISTINCT e.id) AS event_count, COUNT(o.id) AS occurrence_count, s.*, p.priority FROM ics_sources AS s LEFT JOIN ics_source_priorities AS p ON p.ics_source_id = s.id AND p.user_id = $1 LEFT JOIN events AS e ON e.event_source_id = s.id LEFT JOIN event_occurrences AS o ON o.event_id = e.id WHERE s.is_public = true OR s.user_id = $1 GROUP BY s.id",
        user_id
    )
    .fetch_all(&data.conn)
    .await
    .unwrap()
    .into_iter()
    .map(|source| {
        let s = IcsSource::from((RawIcsSource {
            id: source.id,
            name: source.name,
            url: source.url,
            user_id: source.user_id,
            last_fetched_at: source.last_fetched_at,
            is_public: source.is_public,
            created_at: source.created_at,
            persist_events: source.persist_events,
            all_as_allday: source.all_as_allday,
            import_template: source.import_template,
        }, source.priority));
        (s, source.event_count as i64, source.occurrence_count as i64)
    })
    .collect()
}
pub async fn get_source_as_user(
    data: &web::Data<AppState>,
    user_id: Option<i64>,
    id: i32,
) -> Option<IcsSource> {
    sqlx::query!(
        "SELECT s.*, p.priority FROM ics_sources AS s LEFT JOIN ics_source_priorities AS p ON p.ics_source_id = s.id AND p.user_id = $1 WHERE (s.is_public = true OR s.user_id = $1) AND s.id = $2",
        user_id,
        id
    )
    .fetch_optional(&data.conn)
    .await
    .expect("Failed to fetch source from db")
    .map(|source| {
        IcsSource::from((RawIcsSource {
            id: source.id,
            name: source.name,
            url: source.url,
            user_id: source.user_id,
            last_fetched_at: source.last_fetched_at,
            is_public: source.is_public,
            created_at: source.created_at,
            persist_events: source.persist_events,
            all_as_allday: source.all_as_allday,
            import_template: source.import_template,
        }, source.priority))
    })
}

use actix_web::web;

use olmonoko_backend::{
    models::{
        event::remote::RemoteSourceId,
        ics_source::{IcsSource, RawIcsSource},
        user::UserId,
    },
    AppState,
};

pub async fn get_visible_sources(
    data: &web::Data<AppState>,
    user_id: Option<UserId>,
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
            updated_at: source.updated_at.map(i64::from),
            persist_events: source.persist_events,
            all_as_allday: source.all_as_allday,
            import_template: source.import_template,
            file_hash: source.file_hash,
                object_hash: source.object_hash,
        }, source.priority))
    })
    .collect()
}
pub async fn get_visible_sources_with_event_count(
    data: &web::Data<AppState>,
    user_id: Option<UserId>,
) -> Vec<(IcsSource, i64, i64)> {
    // TODO: Check if p.priority has to be considered in grouping
    sqlx::query!(
        "SELECT COUNT(DISTINCT e.id) AS event_count, COUNT(o.id) AS occurrence_count, s.*, MAX(CASE WHEN p.priority IS NOT NULL THEN p.priority END) AS priority FROM ics_sources AS s LEFT JOIN ics_source_priorities AS p ON p.ics_source_id = s.id AND p.user_id = $1 LEFT JOIN events AS e ON e.event_source_id = s.id LEFT JOIN event_occurrences AS o ON o.event_id = e.id WHERE s.is_public = true OR s.user_id = $1 GROUP BY s.id, p.priority",
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
            updated_at: source.updated_at.map(i64::from),
            persist_events: source.persist_events,
            all_as_allday: source.all_as_allday,
            import_template: source.import_template,
            file_hash: source.file_hash,
            object_hash: source.object_hash,
        }, source.priority));
        (s, source.event_count.unwrap_or_default(), source.occurrence_count.unwrap_or_default())
    })
    .collect()
}
pub async fn get_source_as_user(
    data: &web::Data<AppState>,
    user_id: Option<UserId>,
    id: RemoteSourceId,
) -> Option<IcsSource> {
    sqlx::query!(
        r#"SELECT s.*, p.priority AS "priority?" FROM ics_sources AS s LEFT JOIN ics_source_priorities AS p ON p.ics_source_id = s.id AND p.user_id = $1 WHERE (s.is_public = true OR s.user_id = $1) AND s.id = $2"#,
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
            updated_at: source.updated_at,
            persist_events: source.persist_events,
            all_as_allday: source.all_as_allday,
            import_template: source.import_template,
            file_hash: source.file_hash,
                object_hash: source.object_hash,
        }, source.priority))
    })
}
pub async fn get_source_as_user_with_event_count(
    data: &web::Data<AppState>,
    user_id: Option<UserId>,
    id: RemoteSourceId,
) -> (IcsSource, i64, i64) {
    let r = sqlx::query!(
        "SELECT COUNT(DISTINCT e.id) AS event_count, COUNT(o.id) AS occurrence_count, s.*, MAX(CASE WHEN p.priority IS NOT NULL THEN p.priority END) AS priority FROM ics_sources AS s LEFT JOIN ics_source_priorities AS p ON p.ics_source_id = s.id AND p.user_id = $1 LEFT JOIN events AS e ON e.event_source_id = s.id LEFT JOIN event_occurrences AS o ON o.event_id = e.id WHERE (s.is_public = true OR s.user_id = $1) AND s.id = $2 GROUP BY s.id",
        user_id,
        id
    )
    .fetch_one(&data.conn)
    .await
    .expect("Failed to fetch source from db");

    let ics_source = IcsSource::from((
        RawIcsSource {
            id: r.id,
            name: r.name,
            url: r.url,
            user_id: r.user_id,
            last_fetched_at: r.last_fetched_at,
            is_public: r.is_public,
            created_at: r.created_at,
            updated_at: r.updated_at,
            persist_events: r.persist_events,
            all_as_allday: r.all_as_allday,
            import_template: r.import_template,
            file_hash: r.file_hash,
            object_hash: r.object_hash,
        },
        r.priority,
    ));

    (
        ics_source,
        r.event_count.unwrap_or_default(),
        r.occurrence_count.unwrap_or_default(),
    )
}

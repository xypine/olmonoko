use actix_web::{get, web, HttpRequest, HttpResponse, Responder, Scope};

use crate::{
    models::{
        attendance::RawAttendance,
        bills::RawBill,
        event::{
            local::RawLocalEvent,
            remote::{RawRemoteEvent, RawRemoteEventOccurrence},
        },
        ics_source::RawIcsSource,
        public_link::RawPublicLink,
        user::RawUser,
    },
    routes::AppState,
    utils::{
        request::{deauth, EnhancedRequest},
        time::timestamp,
    },
};

use super::meta::BuildInformation;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Backup {
    pub created_at: i64,
    pub site_url: String,
    pub version: String,
    pub build_info: BuildInformation,
    pub users: Vec<RawUser>,
    pub sources: Vec<RawIcsSource>,
    pub source_priorities: Vec<(i64, i64, i64)>, // user_id, ics_source_id, priority
    pub local_events: Vec<RawLocalEvent>,
    pub attendance: Vec<RawAttendance>,
    pub bills: Vec<RawBill>,
    pub public_links: Vec<RawPublicLink>,
    pub persisted_remote_events: Vec<RawRemoteEvent>,
    pub persisted_remote_event_occurrences: Vec<RawRemoteEventOccurrence>,
    pub tags: Vec<(i64, Option<i64>, Option<i64>, String)>, // created_at, local_event_id, remote_event_id, tag
}

#[get("/dump.json")]
async fn export(data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Some(user) = req.get_session_user(&data).await {
        tracing::info!(user.id, user.email, user.admin, "User requested a backup");
        if !user.admin {
            return deauth();
        }
        let users: Vec<RawUser> = sqlx::query_as!(RawUser, "SELECT * FROM users")
            .fetch_all(&data.conn)
            .await
            .expect("Failed to fetch users");
        let sources: Vec<RawIcsSource> = sqlx::query_as!(RawIcsSource, "SELECT * FROM ics_sources")
            .fetch_all(&data.conn)
            .await
            .expect("Failed to fetch sources");
        let source_priorities: Vec<(i64, i64, i64)> =
            sqlx::query!("SELECT * FROM ics_source_priorities")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch source priorities")
                .into_iter()
                .map(|p| (p.user_id, p.ics_source_id, p.priority))
                .collect();
        let persisted_remote_events: Vec<RawRemoteEvent> =
            sqlx::query_as!(RawRemoteEvent, "SELECT events.* FROM events INNER JOIN ics_sources ON events.event_source_id = ics_sources.id AND ics_sources.persist_events = true")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch remote events");
        let persisted_remote_event_occurrences: Vec<RawRemoteEventOccurrence> =
            sqlx::query_as!(RawRemoteEventOccurrence, "SELECT event_occurrences.* FROM event_occurrences INNER JOIN events ON events.id = event_occurrences.event_id INNER JOIN ics_sources ON events.event_source_id = ics_sources.id AND ics_sources.persist_events = true")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch remote event occurrences");
        let local_events: Vec<RawLocalEvent> =
            sqlx::query_as!(RawLocalEvent, "SELECT * FROM local_events")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch local events");
        let tags: Vec<(i64, Option<i64>, Option<i64>, String)> =
            sqlx::query!("SELECT event_tags.* FROM event_tags INNER JOIN events ON events.id = event_tags.remote_event_id INNER JOIN ics_sources ON events.event_source_id = ics_sources.id AND ics_sources.persist_events = true UNION SELECT * FROM event_tags WHERE local_event_id IS NOT NULL")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch event tags")
                .into_iter()
                .map(|t| (t.created_at, t.local_event_id, t.remote_event_id, t.tag))
                .collect();
        let attendance: Vec<RawAttendance> =
            sqlx::query_as!(RawAttendance, "SELECT attendance.* FROM attendance INNER JOIN events ON events.id = attendance.remote_event_id INNER JOIN ics_sources ON events.event_source_id = ics_sources.id AND ics_sources.persist_events = true UNION SELECT * FROM attendance WHERE local_event_id IS NOT NULL")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch local event attendance");
        let bills: Vec<RawBill> = sqlx::query_as!(RawBill, "SELECT bills.* FROM bills INNER JOIN events ON events.id = bills.remote_event_id INNER JOIN ics_sources ON events.event_source_id = ics_sources.id AND ics_sources.persist_events = true UNION SELECT * FROM bills WHERE local_event_id IS NOT NULL")
            .fetch_all(&data.conn)
            .await
            .expect("Failed to fetch bills");
        let public_links: Vec<RawPublicLink> =
            sqlx::query_as!(RawPublicLink, "SELECT * FROM public_calendar_links")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch public links");
        let backup = Backup {
            created_at: timestamp(),
            site_url: data.site_url.clone(),
            version: data.version.clone(),
            build_info: data.build_info.clone(),
            users,
            sources,
            source_priorities,
            persisted_remote_events,
            local_events,
            tags,
            attendance,
            bills,
            public_links,
            persisted_remote_event_occurrences,
        };
        return HttpResponse::Ok().json(backup);
    }
    deauth()
}

pub fn routes() -> Scope {
    web::scope("/backup").service(export)
}

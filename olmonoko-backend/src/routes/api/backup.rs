use actix_web::{get, web, HttpRequest, HttpResponse, Responder, Scope};

use crate::{
    models::{
        attendance::RawAttendance,
        bills::RawBill,
        event::{
            local::{LocalEventId, RawLocalEvent},
            remote::RawRemoteEvent,
            Priority,
        },
        ics_source::{IcsSourceId, RawIcsSource},
        public_link::RawPublicLink,
        user::{RawUser, UserId},
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
    pub source_priorities: Vec<(UserId, IcsSourceId, Priority)>, // user_id, ics_source_id, priority
    pub local_events: Vec<RawLocalEvent>,
    pub local_event_tags: Vec<(LocalEventId, String, i64)>, // local_event_id, tag, created_at
    pub attendance: Vec<RawAttendance>,
    pub bills: Vec<RawBill>,
    pub public_links: Vec<RawPublicLink>,
    pub remote_events: Vec<RawRemoteEvent>,
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
        let source_priorities: Vec<_> = sqlx::query!("SELECT * FROM ics_source_priorities")
            .fetch_all(&data.conn)
            .await
            .expect("Failed to fetch source priorities")
            .into_iter()
            .map(|p| (p.user_id, p.ics_source_id, p.priority))
            .collect();
        let remote_events: Vec<RawRemoteEvent> =
            sqlx::query_as!(RawRemoteEvent, "SELECT * FROM events")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch remote events");
        let local_events: Vec<RawLocalEvent> =
            sqlx::query_as!(RawLocalEvent, "SELECT * FROM local_events")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch local events");
        let local_event_tags: Vec<_> =
            sqlx::query!("SELECT * FROM event_tags WHERE remote_event_id IS NULL")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch local event tags")
                .into_iter()
                .map(|t| (t.local_event_id.unwrap(), t.tag, t.created_at))
                .collect();
        let attendance: Vec<RawAttendance> =
            sqlx::query_as!(RawAttendance, "SELECT * FROM attendance")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to fetch local event attendance");
        let bills: Vec<RawBill> = sqlx::query_as!(RawBill, "SELECT * FROM bills")
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
            remote_events,
            local_events,
            local_event_tags,
            attendance,
            bills,
            public_links,
        };
        return HttpResponse::Ok().json(backup);
    }
    deauth()
}

pub fn routes() -> Scope {
    web::scope("/backup").service(export)
}

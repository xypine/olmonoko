use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Scope};

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
    pub public_links: Vec<RawPublicLink>,
    pub local_events: Vec<RawLocalEvent>,
    pub local_event_tags: Vec<(LocalEventId, String, i64)>, // local_event_id, tag, created_at
    pub sources: Vec<RawIcsSource>,
    pub source_priorities: Vec<(UserId, IcsSourceId, Priority)>, // user_id, ics_source_id, priority
    pub attendance: Vec<RawAttendance>,
    pub bills: Vec<RawBill>,
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

#[post("/restore.json")]
async fn restore(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<Backup>,
) -> impl Responder {
    if let Some(user) = req.get_session_user(&data).await {
        tracing::info!(user.id, user.email, user.admin, "User requested a restore");
        if !user.admin {
            return deauth();
        }
        tracing::info!("=== Backup Metadata ===");
        tracing::info!("Created at: {}", body.created_at);
        tracing::info!("Site URL: {}", body.site_url);
        tracing::info!("Version: {}", body.version);
        tracing::info!("Build info: {:?}", body.build_info);
        tracing::info!("=== Restoring backup ===");
        let mut txn = data
            .conn
            .begin()
            .await
            .expect("Failed to start transaction");
        tracing::info!("Removing all existing users");
        sqlx::query!("DELETE FROM users")
            .execute(&mut *txn)
            .await
            .expect("Failed to delete users");
        tracing::info!("Removing all existing unverified users");
        sqlx::query!("DELETE FROM unverified_users")
            .execute(&mut *txn)
            .await
            .expect("Failed to delete unverified users");

        tracing::info!("Restoring users");
        for user in &body.users {
            sqlx::query!(
                "INSERT INTO users (id, email, password_hash, admin, created_at, interface_timezone) VALUES ($1, $2, $3, $4, $5, $6)",
                user.id,
                user.email,
                user.password_hash,
                user.admin,
                user.created_at,
                user.interface_timezone,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert user");
        }

        tracing::info!("Restoring public links");
        for link in &body.public_links {
            sqlx::query!(
                "INSERT INTO public_calendar_links (id, user_id, created_at, min_priority, max_priority) VALUES ($1, $2, $3, $4, $5)",
                link.id,
                link.user_id,
                link.created_at,
                link.min_priority,
                link.max_priority,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert public link");
        }

        tracing::info!("Restoring local events");
        for event in &body.local_events {
            sqlx::query!(
                "INSERT INTO local_events (id, user_id, created_at, updated_at, starts_at, duration, summary, description, location, uid, all_day, priority) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                event.id,
                event.user_id,
                event.created_at,
                event.updated_at,
                event.starts_at,
                event.duration,
                event.summary,
                event.description,
                event.location,
                event.uid,
                event.all_day,
                event.priority,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert local event");
        }

        tracing::info!("Restoring local event tags");
        for (local_event_id, tag, created_at) in &body.local_event_tags {
            sqlx::query!(
                "INSERT INTO event_tags (local_event_id, tag, created_at) VALUES ($1, $2, $3)",
                local_event_id,
                tag,
                created_at,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert local event tag");
        }

        tracing::info!("Restoring sources");
        // id, user_id, is_public, name, url, created_at, last_fetched_at, file_hash, object_hash, updated_at, persist_events, all_as_allday, import_template
        for source in &body.sources {
            sqlx::query!(
                "INSERT INTO ics_sources (id, user_id, is_public, name, url, created_at, last_fetched_at, file_hash, object_hash, updated_at, persist_events, all_as_allday, import_template) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                source.id,
                source.user_id,
                source.is_public,
                source.name,
                source.url,
                source.created_at,
                source.last_fetched_at,
                source.file_hash,
                source.object_hash,
                source.updated_at,
                source.persist_events,
                source.all_as_allday,
                source.import_template,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert source");
        }

        tracing::info!("Restoring source priorities");
        for (user_id, ics_source_id, priority) in &body.source_priorities {
            sqlx::query!(
                "INSERT INTO ics_source_priorities (user_id, ics_source_id, priority) VALUES ($1, $2, $3)",
                user_id,
                ics_source_id,
                priority,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert source priority");
        }

        // todo: attendance, bills, remote events

        tracing::info!("Committing transaction");
        txn.commit().await.expect("Failed to commit transaction");
        tracing::info!("Restore complete");
        return HttpResponse::Ok().body("Restore complete");
    }
    deauth()
}

pub fn routes() -> Scope {
    let json_cfg = web::JsonConfig::default()
        // raise max json payload size to 100 MB
        .limit(1024 * 1024 * 100);
    web::scope("/backup")
        .app_data(json_cfg)
        .service(export)
        .service(restore)
}

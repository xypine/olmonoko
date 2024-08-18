use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Scope};

use crate::{
    db_utils::request::{deauth, EnhancedRequest, SESSION_COOKIE_NAME},
    middleware::autocacher::CACHE_RECURSION_PREVENTION_HEADER,
};
use olmonoko_common::{
    models::{
        attendance::RawAttendance,
        bills::RawBill,
        event::{
            local::{LocalEventId, RawLocalEvent},
            remote::{RawRemoteEvent, RawRemoteEventOccurrence, RemoteEventId},
            Priority,
        },
        ics_source::{IcsSourceId, RawIcsSource},
        public_link::RawPublicLink,
        user::{RawUser, User, UserId},
    },
    utils::time::timestamp,
    AppState, BuildInformation,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Backup {
    pub created_at: i64,
    pub site_url: String,
    pub version: String,
    pub build_info: BuildInformation,
    pub users: Vec<RawUser>,
    pub public_links: Vec<RawPublicLink>,
    pub local_events: Vec<RawLocalEvent>,
    pub sources: Vec<RawIcsSource>,
    pub source_priorities: Vec<(UserId, IcsSourceId, Priority)>, // user_id, ics_source_id, priority
    pub attendance: Vec<RawAttendance>,
    pub bills: Vec<RawBill>,
    pub persisted_remote_events: Vec<RawRemoteEvent>,
    pub persisted_remote_event_occurrences: Vec<RawRemoteEventOccurrence>,
    pub tags: Vec<(i64, Option<LocalEventId>, Option<RemoteEventId>, String)>, // created_at, local_event_id, remote_event_id, tag
}

#[get("/dump.json")]
async fn export(data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Some(user) = req.get_session_user(&data).await {
        tracing::info!(user.id, user.email, user.admin, "User requested a backup");
        if !user.admin {
            return deauth(&req);
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
        let tags: Vec<_> = sqlx::query!(
            "
                SELECT event_tags.*
                FROM event_tags
                LEFT JOIN events ON events.id = event_tags.remote_event_id
                LEFT JOIN ics_sources ON events.event_source_id = ics_sources.id
                WHERE (ics_sources.persist_events = true OR event_tags.local_event_id IS NOT NULL);
            "
        )
        .fetch_all(&data.conn)
        .await
        .expect("Failed to fetch event tags")
        .into_iter()
        .map(|t| (t.created_at, t.local_event_id, t.remote_event_id, t.tag))
        .collect();
        let attendance: Vec<RawAttendance> = sqlx::query_as!(
            RawAttendance,
            "
                SELECT attendance.*
                FROM attendance
                LEFT JOIN events ON events.id = attendance.remote_event_id
                LEFT JOIN ics_sources ON events.event_source_id = ics_sources.id
                WHERE (ics_sources.persist_events = true OR attendance.local_event_id IS NOT NULL);
            "
        )
        .fetch_all(&data.conn)
        .await
        .expect("Failed to fetch local event attendance");
        let bills: Vec<RawBill> = sqlx::query_as!(
            RawBill,
            "
                SELECT bills.*
                FROM bills
                LEFT JOIN events ON events.id = bills.remote_event_id
                LEFT JOIN ics_sources ON events.event_source_id = ics_sources.id
                WHERE (ics_sources.persist_events = true OR bills.local_event_id IS NOT NULL);
            "
        )
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
    deauth(&req)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct InstanceCloneForm {
    instance_url: String,
    session_id: String,
}

#[post("/clone")]
async fn clone_instance(
    data: web::Data<AppState>,
    req: HttpRequest,
    form: web::Form<InstanceCloneForm>,
) -> impl Responder {
    if let Some(user) = req.get_session_user(&data).await {
        if !user.admin {
            return deauth(&req);
        }
        let session_id = form.session_id.clone();
        let instance_endpoint = format!("{}/api/backup/dump.json", form.instance_url);
        let cookies = reqwest::cookie::Jar::default();
        cookies.add_cookie_str(
            &format!("{}={}", SESSION_COOKIE_NAME, session_id),
            &reqwest::Url::parse(&form.instance_url).unwrap(),
        );
        let client = reqwest::Client::builder()
            .cookie_provider(std::sync::Arc::new(cookies))
            .build()
            .unwrap();
        let response = client
            .get(&instance_endpoint)
            .header(CACHE_RECURSION_PREVENTION_HEADER, "true")
            .send()
            .await
            .unwrap();
        if !response.status().is_success() {
            tracing::warn!("Failed to fetch instance backup: {}", response.status());
            return HttpResponse::InternalServerError().body("failed to fetch instance backup");
        }
        let body: Backup = response.json().await.unwrap();
        if !restore(data, &user, body).await.unwrap() {
            return deauth(&req);
        }
        return HttpResponse::Ok().body("Clone complete!");
    }
    return deauth(&req);
}

async fn restore(
    data: web::Data<AppState>,
    user: &User,
    body: Backup,
) -> Result<bool, sqlx::Error> {
    tracing::info!(user.id, user.email, user.admin, "User requested a restore");
    if !user.admin {
        return Ok(false);
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

    tracing::info!("Restoring sources");
    for source in &body.sources {
        // Restoring file or object hashes would block updates to the source until the file changes
        let file_hash: Option<String> = None;
        let object_hash: Option<String> = None;

        sqlx::query!(
                "INSERT INTO ics_sources (id, user_id, is_public, name, url, created_at, last_fetched_at, file_hash, object_hash, updated_at, persist_events, all_as_allday, import_template) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                source.id,
                source.user_id,
                source.is_public,
                source.name,
                source.url,
                source.created_at,
                source.last_fetched_at,
                file_hash,
                object_hash,
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

    tracing::info!("Restoring remote events");
    for event in &body.persisted_remote_events {
        sqlx::query!(
                "INSERT INTO events (id, event_source_id, priority_override, rrule, dt_stamp, all_day, duration, summary, description, location, uid) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                event.id,
                event.event_source_id,
                event.priority_override,
                event.rrule,
                event.dt_stamp,
                event.all_day,
                event.duration,
                event.summary,
                event.description,
                event.location,
                event.uid,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert remote event");
    }

    tracing::info!("Restoring remote event occurrences");
    for occurrence in &body.persisted_remote_event_occurrences {
        sqlx::query!(
                "INSERT INTO event_occurrences (id, event_id, starts_at, from_rrule) VALUES ($1, $2, $3, $4)",
                occurrence.id,
                occurrence.event_id,
                occurrence.starts_at,
                occurrence.from_rrule,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert remote event occurrence");
    }

    tracing::info!("Restoring event tags");
    for (created_at, local_event_id, remote_event_id, tag) in &body.tags {
        sqlx::query!(
                "INSERT INTO event_tags (created_at, local_event_id, remote_event_id, tag) VALUES ($1, $2, $3, $4)",
                created_at,
                *local_event_id,
                *remote_event_id,
                tag,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert local event tag");
    }

    tracing::info!("Restoring attendance");
    for attendance in &body.attendance {
        sqlx::query!(
                "INSERT INTO attendance (user_id, local_event_id, remote_event_id, planned, actual, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                attendance.user_id,
                attendance.local_event_id,
                attendance.remote_event_id,
                attendance.planned,
                attendance.actual,
                attendance.created_at,
                attendance.updated_at,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert attendance");
    }

    tracing::info!("Restoring bills");
    for bill in &body.bills {
        sqlx::query!(
                "INSERT INTO bills (id, local_event_id, remote_event_id, payee_account_number, amount, reference, payee_name, payee_email, payee_address, payee_phone, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                bill.id,
                bill.local_event_id,
                bill.remote_event_id,
                bill.payee_account_number,
                bill.amount,
                bill.reference,
                bill.payee_name,
                bill.payee_email,
                bill.payee_address,
                bill.payee_phone,
                bill.created_at,
                bill.updated_at,
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert bill");
    }

    // NOTE See https://wiki.postgresql.org/wiki/Fixing_Sequences
    tracing::info!("Resyncing Sequences");
    let statements: Vec<String> = sqlx::query_scalar(r#"SELECT
    'SELECT SETVAL(' ||
       quote_literal(quote_ident(sequence_namespace.nspname) || '.' || quote_ident(class_sequence.relname)) ||
       ', COALESCE(MAX(' ||quote_ident(pg_attribute.attname)|| '), 1) ) FROM ' ||
       quote_ident(table_namespace.nspname)|| '.'||quote_ident(class_table.relname)|| ';'
FROM pg_depend
    INNER JOIN pg_class AS class_sequence
        ON class_sequence.oid = pg_depend.objid
            AND class_sequence.relkind = 'S'
    INNER JOIN pg_class AS class_table
        ON class_table.oid = pg_depend.refobjid
    INNER JOIN pg_attribute
        ON pg_attribute.attrelid = class_table.oid
            AND pg_depend.refobjsubid = pg_attribute.attnum
    INNER JOIN pg_namespace as table_namespace
        ON table_namespace.oid = class_table.relnamespace
    INNER JOIN pg_namespace AS sequence_namespace
        ON sequence_namespace.oid = class_sequence.relnamespace
ORDER BY sequence_namespace.nspname, class_sequence.relname;"#)
            .fetch_all(&mut *txn)
            .await
            .expect("Failed to query sequences to resync");
    for statement in statements {
        println!("executing statement: {}", statement);
        sqlx::query(&statement)
            .execute(&mut *txn)
            .await
            .expect("Failed to resync sequences");
    }

    tracing::info!("Scheduling post-restore sync");
    crate::logic::scheduler::schedule_sync_oneoff(&data.scheduler)
        .await
        .expect("Failed to schedule post-restore sync!");

    tracing::info!("Committing transaction");
    txn.commit().await.expect("Failed to commit transaction");
    tracing::info!("Restore complete!");
    return Ok(true);
}

#[post("/restore.json")]
async fn restore_json(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<Backup>,
) -> impl Responder {
    if let Some(user) = req.get_session_user(&data).await {
        let allowed = restore(data, &user, body.into_inner())
            .await
            .expect("Failed to restore backup");
        if !allowed {
            return deauth(&req);
        }
        return HttpResponse::Ok().body("Restore complete");
    }
    deauth(&req)
}

pub fn routes() -> Scope {
    let json_cfg = web::JsonConfig::default()
        // raise max json payload size to 100 MB
        .limit(1024 * 1024 * 100);
    web::scope("/backup")
        .app_data(json_cfg)
        .service(export)
        .service(restore_json)
        .service(clone_instance)
}

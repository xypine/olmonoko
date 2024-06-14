use actix_web::{delete, patch, HttpRequest};
use actix_web::{get, post, web, HttpResponse, Responder, Scope};

use crate::logic::source_processing::sync_source;
use crate::models::ics_source::{IcsSource, IcsSourceForm, NewIcsSource, RawIcsSource};
use crate::routes::api::user::{deauth, get_user_from_request, reload};
use crate::routes::ui::get_session_context;
use crate::routes::AppState;
use crate::utils::flash::{FlashMessage, WithFlashMessage};
use crate::utils::time::timestamp;

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

#[get("")]
async fn sources(data: web::Data<AppState>, request: HttpRequest) -> impl Responder {
    let user_id = get_user_from_request(&data, &request).await.map(|u| u.id);
    let sources = get_visible_sources(&data, user_id).await;
    HttpResponse::Ok().json(sources)
}

#[get("/{id}")]
async fn source_by_id(
    data: web::Data<AppState>,
    path: web::Path<i32>,
    request: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();
    let user_id = get_user_from_request(&data, &request).await.map(|u| u.id);
    let source = get_source_as_user(&data, user_id, id).await;
    HttpResponse::Ok().json(source)
}

const MIN_NAME_LENGTH: usize = 3;
#[post("")]
async fn create_source(
    data: web::Data<AppState>,
    source: web::Form<IcsSourceForm>,
    request: HttpRequest,
) -> impl Responder {
    if source.name.len() < MIN_NAME_LENGTH {
        return reload(&request)
            .with_flash_message(FlashMessage::error(
                format!("Name must be at least {} characters", MIN_NAME_LENGTH).as_str(),
            ))
            .finish();
    }
    if let Some(user) = get_user_from_request(&data, &request).await {
        let mut active_source = NewIcsSource {
            name: source.name.clone(),
            url: source.url.clone(),
            is_public: source.is_public,
            user_id: 0,            // Placeholder
            last_fetched_at: None, // Placeholder
            import_template: source.import_template.clone(),
        };
        active_source.user_id = user.id;
        active_source.last_fetched_at = Some(timestamp());

        let mut txn = data
            .conn
            .begin()
            .await
            .expect("Failed to start transaction");
        let inserted_id = sqlx::query_scalar!("INSERT INTO ics_sources (name, url, user_id, last_fetched_at, is_public, import_template) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id", active_source.name, active_source.url, active_source.user_id, active_source.last_fetched_at, active_source.is_public, active_source.import_template)
            .fetch_one(&mut *txn)
            .await
            .expect("Failed to insert source");
        if let Err(e) = sync_source(&mut *txn, inserted_id).await {
            txn.rollback()
                .await
                .expect("Failed to rollback transaction");
            return reload(&request)
                .with_flash_message(FlashMessage::error(
                    format!("Failed to sync: {}", e).as_str(),
                ))
                .finish();
        }
        txn.commit().await.expect("Failed to commit transaction");
        return reload(&request)
            .with_flash_message(FlashMessage::info("Source added"))
            .finish();
    }
    deauth()
}

#[delete("/{id}")]
async fn delete_source(
    data: web::Data<AppState>,
    path: web::Path<i32>,
    request: HttpRequest,
) -> impl Responder {
    if let Some(user) = get_user_from_request(&data, &request).await {
        let id = path.into_inner();
        sqlx::query!(
            "DELETE FROM ics_sources WHERE id = $1 AND user_id = $2",
            id,
            user.id
        )
        .execute(&data.conn)
        .await
        .expect("Failed to delete source");
        return HttpResponse::Ok().body("Deleted");
    }
    deauth()
}

use serde_with::As;
use serde_with::NoneAsEmptyString;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangePriorityForm {
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub priority: Option<i64>,
}
#[patch("/{id}/priority")]
async fn change_priority(
    data: web::Data<AppState>,
    path: web::Path<i32>,
    form: web::Form<ChangePriorityForm>,
    request: HttpRequest,
) -> impl Responder {
    let (mut context, user_opt) = get_session_context(&data, &request).await;
    if let Some(user) = user_opt {
        let id = path.into_inner();
        let form = form.into_inner();
        if let Some(priority) = form.priority {
            sqlx::query!(
                "INSERT INTO ics_source_priorities (user_id, ics_source_id, priority) VALUES ($1, $2, $3) ON CONFLICT (user_id, ics_source_id) DO UPDATE SET priority = $3",
                user.id,
                id,
                priority
            )
            .execute(&data.conn)
            .await
            .expect("Failed to update priority");
        } else {
            sqlx::query!(
                "DELETE FROM ics_source_priorities WHERE user_id = $1 AND ics_source_id = $2",
                user.id,
                id
            )
            .execute(&data.conn)
            .await
            .expect("Failed to delete priority");
        }
        context.insert(
            "source",
            &IcsSource {
                user_id: user.id,
                id: id as i64,
                chosen_priority: form.priority,
                is_public: false,
                url: "".to_string(),
                name: "".to_string(),
                created_at: chrono::Utc::now(),
                last_fetched_at: None,
                persist_events: false,
                all_as_allday: false,
                import_template: None,
            },
        );
        let component = data
            .templates
            .render("components/data_source/priority_selector.html", &context)
            .unwrap();
        return HttpResponse::Ok().body(component);
    }
    deauth()
}

use crate::models::ics_source::deserialize_checkbox;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeEventPersistenceForm {
    #[serde(deserialize_with = "deserialize_checkbox", default)]
    pub persist: bool,
}
#[patch("/{id}/persist_events")]
async fn change_persist_events(
    data: web::Data<AppState>,
    path: web::Path<i32>,
    form: web::Form<ChangeEventPersistenceForm>,
    request: HttpRequest,
) -> impl Responder {
    let (mut context, user_opt) = get_session_context(&data, &request).await;
    if let Some(user) = user_opt {
        let id = path.into_inner();
        let form = form.into_inner();
        let new_value = sqlx::query_scalar!(
            "UPDATE ics_sources SET persist_events = $1 WHERE id = $2 AND user_id = $3 RETURNING persist_events",
            form.persist,
            id,
            user.id
        )
        .fetch_one(&data.conn)
        .await
        .expect("Failed to update persist events");
        context.insert(
            "source",
            &IcsSource {
                user_id: user.id,
                id: id as i64,
                chosen_priority: None,
                is_public: false,
                url: "".to_string(),
                name: "".to_string(),
                created_at: chrono::Utc::now(),
                last_fetched_at: None,
                persist_events: new_value,
                all_as_allday: false,
                import_template: None,
            },
        );
        let component = data
            .templates
            .render("components/data_source/persist_setting.html", &context)
            .unwrap();
        return HttpResponse::Ok().body(component);
    }
    deauth()
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeAllAsAlldayForm {
    #[serde(deserialize_with = "deserialize_checkbox", default)]
    pub all_as_allday: bool,
}
#[patch("/{id}/all_as_allday")]
async fn change_all_as_allday(
    data: web::Data<AppState>,
    path: web::Path<i32>,
    form: web::Form<ChangeAllAsAlldayForm>,
    request: HttpRequest,
) -> impl Responder {
    let (mut context, user_opt) = get_session_context(&data, &request).await;
    if let Some(user) = user_opt {
        let id = path.into_inner();
        let form = form.into_inner();
        let new_value = sqlx::query_scalar!(
            "UPDATE ics_sources SET all_as_allday = $1 WHERE id = $2 AND user_id = $3 RETURNING all_as_allday",
            form.all_as_allday,
            id,
            user.id
        )
        .fetch_one(&data.conn)
        .await
        .expect("Failed to update all as allday");
        context.insert(
            "source",
            &IcsSource {
                user_id: user.id,
                id: id as i64,
                chosen_priority: None,
                is_public: false,
                url: "".to_string(),
                name: "".to_string(),
                created_at: chrono::Utc::now(),
                last_fetched_at: None,
                persist_events: false,
                all_as_allday: new_value,
                import_template: None,
            },
        );
        let component = data
            .templates
            .render(
                "components/data_source/all_as_allday_setting.html",
                &context,
            )
            .unwrap();
        return HttpResponse::Ok().body(component);
    }
    deauth()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeImportTemplateForm {
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub import_template: Option<String>,
}
#[patch("/{id}/import_template")]
async fn change_import_template(
    data: web::Data<AppState>,
    path: web::Path<i32>,
    form: web::Form<ChangeImportTemplateForm>,
    request: HttpRequest,
) -> impl Responder {
    let (mut context, user_opt) = get_session_context(&data, &request).await;
    if let Some(user) = user_opt {
        let id = path.into_inner();
        let form = form.into_inner();
        let new_value = sqlx::query_scalar!(
            "UPDATE ics_sources SET import_template = $1 WHERE id = $2 AND user_id = $3 RETURNING import_template",
            form.import_template,
            id,
            user.id
        )
        .fetch_one(&data.conn)
        .await
        .expect("Failed to update import template");
        context.insert(
            "source",
            &IcsSource {
                user_id: user.id,
                id: id as i64,
                chosen_priority: None,
                is_public: false,
                url: "".to_string(),
                name: "".to_string(),
                created_at: chrono::Utc::now(),
                last_fetched_at: None,
                persist_events: false,
                all_as_allday: false,
                import_template: new_value,
            },
        );
        let component = data
            .templates
            .render(
                "components/data_source/import_template_setting.html",
                &context,
            )
            .unwrap();
        return HttpResponse::Ok().body(component);
    }
    deauth()
}

#[post("/{id}/sync")]
async fn force_sync(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    request: HttpRequest,
) -> impl Responder {
    if (get_user_from_request(&data, &request).await).is_none() {
        return deauth();
    }
    let id = path.into_inner();
    let mut txn = data
        .conn
        .begin()
        .await
        .expect("Failed to start transaction");
    sync_source(&mut *txn, id)
        .await
        .expect("Failed to sync source");
    txn.commit().await.expect("Failed to commit transaction");

    reload(&request)
        .with_flash_message(FlashMessage::info("Synced successfully"))
        .finish()
}

pub fn routes() -> Scope {
    web::scope("/source")
        .service(sources)
        .service(source_by_id)
        .service(create_source)
        .service(delete_source)
        .service(change_priority)
        .service(change_persist_events)
        .service(change_all_as_allday)
        .service(change_import_template)
        .service(force_sync)
}

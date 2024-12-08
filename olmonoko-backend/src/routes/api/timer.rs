use actix_web::HttpRequest;
use actix_web::{post, web, HttpResponse, Responder, Scope};
use olmonoko_common::models::attendance::NewAttendance;
use olmonoko_common::models::event::local::{LocalEvent, NewLocalEvent, RawLocalEvent};
use olmonoko_common::models::event::Priority;
use olmonoko_common::models::timer::{NewTimer, RawTimer, Timer, TimerForm, TimerId};
use olmonoko_common::utils::time::timestamp;

use crate::db_utils::attendance::DBWrite;
use crate::db_utils::errors::TemplateOrDatabaseError;
use crate::db_utils::request::{reload, AnyInternalServerError, EnhancedRequest, OrInternalServerError};
use olmonoko_common::AppState;

#[post("")]
async fn start(
    form: web::Form<TimerForm>,
    data: web::Data<AppState>,
    request: HttpRequest,
) -> Result<impl Responder, AnyInternalServerError> {
    let (mut context, user_opt, _key, _timer) = request.get_session_context(&data).await;

    if let Some(user) = user_opt {
        let details = NewTimer::from(form.into_inner());
        let result_raw = sqlx::query_as!(RawTimer, r#"INSERT INTO timers (user_id, template, summary, details, location, created_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"#, user.id, details.template, details.summary, details.details, details.location, details.created_at).fetch_one(&data.conn).await;
        let result = match result_raw {
            Ok(result_raw) => {
                Timer::from(result_raw)
            },
            Err(sqlx::Error::Database(e)) => {
                let msg = e.message();
                if msg == "olmonoko.timer.forbidden-template" {
                    return Ok(HttpResponse::Forbidden().body("You are not authorized to use that event as a timer template"));
                }
                println!("{msg}");
                Err(e).or_any_internal_server_error("Failed to insert new timer into db")?
            },
            Err(e) => Err(e).or_any_internal_server_error("Failed to insert new timer into db")?
        };
        if request.is_frontend_request() {
            context.insert("timer_active", &result);

            let content = data
                .templates
                .render("components/timer.html", &context)
                .map_err(TemplateOrDatabaseError::from)
                .or_any_internal_server_error("Failed to render template")?;
            return Ok(HttpResponse::Ok().body(content));
        } else {
            return Ok(HttpResponse::Ok().json(result));
        }
    }
    Ok(HttpResponse::Unauthorized().finish())
}

// Always in the past, not that important overall
const DEFAULT_TIMER_PRIORITY: Priority = 9;

#[post("/{id}/stop")]
async fn stop(
    data: web::Data<AppState>,
    path: web::Path<TimerId>,
    request: HttpRequest,
) -> Result<impl Responder, AnyInternalServerError> {
    let (_context, user_opt, _key, _timer) = request.get_session_context(&data).await;

    if let Some(user) = &user_opt {
        let timer_id = path.into_inner();
        let ends_at = timestamp();

        let timer = sqlx::query_as!(RawTimer, "SELECT * FROM timers WHERE user_id = $1 AND id = $2", user.id, timer_id).fetch_one(&data.conn).await.or_any_internal_server_error("Failed to fetch timer").map(Timer::from)?;
        let duration = ends_at - timer.created_at.timestamp();
        let (summary, details, location, priority, tags) = {
            let template = sqlx::query_as!(RawLocalEvent, "SELECT * FROM local_events WHERE user_id = $1 AND id = $2", user.id, timer.template).fetch_one(&data.conn).await.or_any_internal_server_error("Failed to fetch timer template")?;
            (timer.summary.unwrap_or(template.summary), timer.details.or(template.description), timer.location.or(template.location), template.priority.or(Some(DEFAULT_TIMER_PRIORITY)), vec!["olmonoko::timer".to_owned()])
        };

        let new = NewLocalEvent {
            user_id: user.id,
            uid: format!("olmonoko::timer::{timer_id}"),
            summary,
            all_day: false,
            location,
            description: details,
            tags,
            priority,
            duration: Some(duration as i32),
            starts_at: timer.created_at.timestamp(),
        };

        // begin transaction
        let mut txn = data
            .conn
            .begin()
            .await
            .or_any_internal_server_error("Failed to begin transaction")?;
        // insert event
        let inserted = sqlx::query_as!(
            RawLocalEvent,
            r#"
                INSERT INTO local_events (user_id, priority, starts_at, all_day, duration, summary, description, location, uid)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING *
            "#,
            new.user_id,
            new.priority,
            new.starts_at,
            new.all_day,
            new.duration,
            new.summary,
            new.description,
            new.location,
            new.uid
        )
            .fetch_one(&mut *txn)
            .await
            .map(LocalEvent::from)
            .expect("Failed to insert new local event");
        // insert tags
        for tag in new.tags {
            sqlx::query!(
                "INSERT INTO event_tags (local_event_id, tag) VALUES ($1, $2)",
                inserted.id,
                tag
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert tag");
        }
        // insert attendance
        let attendance = NewAttendance {
            user_id: user.id,
            actual: true,
            planned: true,
            event_id: olmonoko_common::models::attendance::AttendanceEvent::Local(inserted.id),
        };
        attendance
            .write(&mut *txn)
            .await
            .expect("Failed to insert attendance");


        sqlx::query!("DELETE FROM timers WHERE user_id = $1 AND id = $2", user.id, timer_id).execute(&mut *txn).await.or_any_internal_server_error("Failed to delete timer")?;

        // commit transaction
        txn.commit().await.expect("Failed to commit transaction");

        if request.is_frontend_request() {
            return Ok(reload(&request, true).finish());
        } else {
            return Ok(HttpResponse::Ok().json(inserted));
        }
    }
    Ok(HttpResponse::Unauthorized().finish())
}

pub fn routes() -> Scope {
    web::scope("/timer").service(start).service(stop)
}

use actix_web::{
    post,
    web::{self, Path},
    HttpRequest, HttpResponse, Responder, Scope,
};

use crate::{
    models::{
        attendance::{AttendanceFormWithUserEventTz, NewAttendance},
        bills::{
            from_barcode::{NewBillBarcodeForm, NewBillBarcodeFormWithUserId},
            EventId, NewBillWithEvent,
        },
        event::{
            local::{LocalEvent, LocalEventForm, LocalEventId, NewLocalEvent, RawLocalEvent},
            DEFAULT_PRIORITY,
        },
    },
    routes::AppState,
    utils::{
        event_filters::{EventFilter, RawEventFilter},
        events::parse_priority,
        flash::{FlashMessage, WithFlashMessage},
        request::{reload, EnhancedRequest},
    },
};

#[post("/local")]
async fn new_local_event(
    data: web::Data<AppState>,
    form: web::Form<LocalEventForm>,
    request: HttpRequest,
) -> impl Responder {
    tracing::info!("Creating new local event: {:?}", form);
    let user_opt = request.get_session_user(&data).await;
    if let Some(user) = user_opt {
        let form = form.into_inner();
        let attendance_form = form.attendance.clone();
        let form_tz = form.starts_at_tz.unwrap_or(user.interface_timezone_h);

        let new = NewLocalEvent::from((form, &user));

        // begin transaction
        let mut txn = data
            .conn
            .begin()
            .await
            .expect("Failed to begin transaction");
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
        let attendance_params: AttendanceFormWithUserEventTz =
            (attendance_form, &user, inserted.id, new.starts_at, form_tz);
        let attendance: NewAttendance = NewAttendance::try_from(attendance_params).unwrap();
        attendance
            .write(&mut *txn)
            .await
            .expect("Failed to insert attendance");
        // commit transaction
        txn.commit().await.expect("Failed to commit transaction");

        return reload(&request)
            .with_flash_message(FlashMessage::info(&format!(
                "Event {} created",
                inserted.id
            )))
            .finish();
    }
    HttpResponse::Unauthorized().finish()
}

#[derive(Debug, serde::Deserialize)]
struct DeleteQuery {
    id: Option<i32>,
    #[serde(flatten)]
    filter: RawEventFilter,
}
#[post("/local/delete")]
async fn delete_local_event(
    data: web::Data<AppState>,
    request: HttpRequest,
    query: web::Query<DeleteQuery>,
) -> impl Responder {
    let user_opt = request.get_session_user(&data).await;
    if let Some(user) = user_opt {
        let query = query.into_inner();
        let filter = EventFilter::from(query.filter);
        let min_priority = parse_priority(filter.min_priority);
        let max_priority = parse_priority(filter.max_priority);
        // let tags = filter.tags.map(|tags| tags.join(","));
        // let exclude_tags = filter.exclude_tags.map(|tags| tags.join(","));
        let deleted = sqlx::query_as!(
            RawLocalEvent,
            r#"
                DELETE FROM local_events
                WHERE user_id = $2::integer
                    AND ($1::integer IS NULL OR id = $1) 
                    AND ($3::bigint IS NULL OR starts_at > $3) 
                    AND ($4::bigint IS NULL OR starts_at < $4) 
                    AND (COALESCE(NULLIF(priority, 0), $7) >= $5 OR $5 IS NULL)
                    AND (COALESCE(NULLIF(priority, 0), $7) <= $6 OR $6 IS NULL)
                    AND ($8::text IS NULL OR summary LIKE $8)
                    AND ($9::text[] IS NULL OR (
                        SELECT tag.tag
                        FROM event_tags AS tag
                        WHERE tag.local_event_id = id
                        AND tag.tag = ANY($9)
                    ) IS NOT NULL)
                    AND ($10::text[] IS NULL OR (
                        SELECT tag.tag
                        FROM event_tags AS tag
                        WHERE tag.local_event_id = id
                        AND tag.tag = ANY($10)
                    ) IS NULL)
                RETURNING *
            "#,
            query.id,
            user.id,
            filter.after,
            filter.before,
            min_priority,
            max_priority,
            DEFAULT_PRIORITY,
            filter.summary_like,
            filter.tags.as_deref(),
            filter.exclude_tags.as_deref(),
        )
        .fetch_all(&data.conn)
        .await
        .expect("Failed to delete local event")
        .into_iter()
        .map(LocalEvent::from)
        .collect::<Vec<_>>();

        let message = if deleted.is_empty() {
            FlashMessage::warning("No events deleted")
        } else if deleted.len() == 1 {
            FlashMessage::info(&format!("Deleted event {}", deleted[0].id))
        } else {
            FlashMessage::info(&format!("Deleted {} event(s)", deleted.len()))
        };
        return reload(&request).with_flash_message(message).finish();
    }
    HttpResponse::Unauthorized().finish()
}

#[post("/local/{id}/update")]
async fn update_local_event(
    data: web::Data<AppState>,
    request: HttpRequest,
    id: Path<LocalEventId>,
    form: web::Form<LocalEventForm>,
) -> impl Responder {
    let user_opt = request.get_session_user(&data).await;
    if let Some(user) = user_opt {
        let id = id.into_inner();
        let form = form.into_inner();
        let attendance_form = form.attendance.clone();
        let form_tz = form.starts_at_tz.unwrap_or(user.interface_timezone_h);

        // begin transaction
        let mut txn = data
            .conn
            .begin()
            .await
            .expect("Failed to begin transaction");
        // update event
        let new = NewLocalEvent::from((form, &user));
        sqlx::query!(
            r#"
                UPDATE local_events
                SET starts_at = $1, all_day = $2, duration = $3, summary = $4, description = $5, location = $6, priority = $7
                WHERE id = $8 AND user_id = $9
            "#,
            new.starts_at,
            new.all_day,
            new.duration,
            new.summary,
            new.description,
            new.location,
            new.priority,
            id,
            user.id
        )
        .execute(&mut *txn)
        .await
        .expect("Failed to update local event");
        // remove all previous tags
        sqlx::query!("DELETE FROM event_tags WHERE local_event_id = $1", id)
            .execute(&mut *txn)
            .await
            .expect("Failed to delete tags");
        // insert new tags
        for tag in new.tags {
            sqlx::query!(
                "INSERT INTO event_tags (local_event_id, tag) VALUES ($1, $2)",
                id,
                tag
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert tag");
        }
        // update attendance
        let attendance_params: AttendanceFormWithUserEventTz =
            (attendance_form, &user, id, new.starts_at, form_tz);
        let attendance: NewAttendance = NewAttendance::try_from(attendance_params).unwrap();
        attendance
            .write(&mut *txn)
            .await
            .expect("Failed to upsert attendance");

        // commit transaction
        txn.commit().await.expect("Failed to commit transaction");

        return reload(&request)
            .with_flash_message(FlashMessage::info(&format!("Event {} updated", id)))
            .finish();
    }
    HttpResponse::Unauthorized().finish()
}

#[post("/bill/from_barcode")]
async fn new_bill_from_barcode(
    data: web::Data<AppState>,
    form: web::Form<NewBillBarcodeForm>,
    request: HttpRequest,
) -> impl Responder {
    let user_opt = request.get_session_user(&data).await;
    if let Some(user) = user_opt {
        let form = form.into_inner();
        let with_user_id = NewBillBarcodeFormWithUserId {
            user_id: user.id,
            form,
        };
        let (new_event, mut new_bill) =
            NewBillWithEvent::try_from(with_user_id).expect("Failed to decode barcode");
        let mut txn = data
            .conn
            .begin()
            .await
            .expect("Failed to begin transaction");
        let inserted_event = sqlx::query_as!(
            RawLocalEvent,
            r#"
                INSERT INTO local_events (user_id, priority, starts_at, all_day, duration, summary, description, location, uid)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING *
            "#,
            new_event.user_id,
            new_event.priority,
            new_event.starts_at,
            new_event.all_day,
            new_event.duration,
            new_event.summary,
            new_event.description,
            new_event.location,
            new_event.uid
        )
            .fetch_one(&mut *txn)
            .await
            .map(LocalEvent::from)
            .expect("Failed to insert new local event");

        // insert tags
        for tag in new_event.tags {
            sqlx::query!(
                "INSERT INTO event_tags (local_event_id, tag) VALUES ($1, $2)",
                inserted_event.id,
                tag
            )
            .execute(&mut *txn)
            .await
            .expect("Failed to insert tag");
        }

        new_bill.event_id = EventId::Local(inserted_event.id);
        sqlx::query!("INSERT INTO bills (local_event_id, payee_account_number, amount, reference, payee_name, payee_email, payee_address, payee_phone) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)", inserted_event.id, new_bill.payee_account_number, new_bill.amount, new_bill.reference, new_bill.payee_name, new_bill.payee_email, new_bill.payee_address, new_bill.payee_phone).execute(&mut *txn).await.expect("Failed to insert new bill");

        txn.commit().await.expect("Failed to commit transaction");

        return reload(&request)
            .with_flash_message(FlashMessage::info(&format!(
                "Event {} created",
                inserted_event.id
            )))
            .finish();
    }
    HttpResponse::Unauthorized().finish()
}

pub fn routes() -> Scope {
    web::scope("/event")
        .service(new_local_event)
        .service(delete_local_event)
        .service(update_local_event)
        .service(new_bill_from_barcode)
}

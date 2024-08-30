use actix_web::{delete, get, patch, post, web, HttpRequest, HttpResponse, Responder, Scope};
use uuid::Uuid;

use crate::db_utils::errors::TemplateOrDatabaseError;
use crate::db_utils::events::{get_user_local_events, get_visible_event_occurrences};
use crate::db_utils::request::{
    deauth, EnhancedRequest, InternalServerError, IntoInternalServerError, OrInternalServerError,
};
use crate::db_utils::user::get_user_export_links;
use olmonoko_common::models::event::{Event, EventOccurrence, Priority};
use olmonoko_common::models::public_link::{PublicLink, RawPublicLink};
use olmonoko_common::utils::event_filters::EventFilter;
use olmonoko_common::AppState;

#[get("/{id}.ics")]
async fn get_calendar(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, InternalServerError<sqlx::Error>> {
    let id = path.into_inner().to_string();
    tracing::info!("Fetching calendar for id {id}");
    let opt = sqlx::query_as!(
        RawPublicLink,
        "SELECT * FROM public_calendar_links WHERE id = $1",
        id
    )
    .fetch_optional(&data.conn)
    .await
    .or_internal_server_error("Failed to fetch public calendar link from the database")?
    .map(PublicLink::from);
    if let Some(public_link) = opt {
        let events = get_visible_event_occurrences(
            &data,
            Some(public_link.user_id),
            true,
            &EventFilter {
                min_priority: public_link.min_priority,
                max_priority: public_link.max_priority,
                ..Default::default()
            },
        )
        .await;
        let ics = crate::logic::compose_ics(events)
            .await
            .expect("Failed to compose ics");
        return Ok(HttpResponse::Ok().content_type("text/calendar").body(ics));
    } else {
        Ok(HttpResponse::NotFound().body("link not found"))
    }
}

// TODO: Fix
//#[get("/local.ics")]
//async fn get_local_calendar(
//    data: web::Data<AppState>,
//    req: HttpRequest,
//) -> Result<impl Responder, InternalServerError<sqlx::Error>> {
//    let user_opt = req.get_session_user(&data).await;
//    if let Some(user) = user_opt {
//        tracing::info!("Fetching local calendar for user {}", user.id);
//        let events: Vec<EventOccurrence> = get_user_local_events(
//            &data,
//            user.id,
//            true,
//            &EventFilter {
//                ..Default::default()
//            },
//        )
//        .await
//        .into_iter()
//        .flat_map(|e| {
//            let o: Vec<EventOccurrence> = e.into();
//            o
//        })
//        .collect();
//        let ics = crate::logic::compose_ics(events)
//            .await
//            .expect("Failed to compose ics");
//        return Ok(HttpResponse::Ok().content_type("text/calendar").body(ics));
//    }
//    Ok(deauth(&req))
//}

#[delete("/{id}.ics")]
async fn delete_link(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    request: HttpRequest,
) -> Result<impl Responder, InternalServerError<sqlx::Error>> {
    let id = path.into_inner().to_string();
    let user_opt = request.get_session_user(&data).await;
    if let Some(user) = user_opt {
        sqlx::query!(
            "DELETE FROM public_calendar_links WHERE id = $1 AND user_id = $2",
            id,
            user.id
        )
        .execute(&data.conn)
        .await
        .or_internal_server_error("Failed to delete public link")?;
        Ok(HttpResponse::Ok().body("link deleted"))
    } else {
        Ok(deauth(&request))
    }
}

use serde_with::As;
use serde_with::NoneAsEmptyString;
#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct ChangePriorityFilterForm {
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    min_priority: Option<Priority>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    max_priority: Option<Priority>,
}
#[patch("/{id}.ics")]
async fn change_filters(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    form: web::Form<ChangePriorityFilterForm>,
    request: HttpRequest,
) -> Result<impl Responder, InternalServerError<TemplateOrDatabaseError>> {
    let id = path.into_inner().to_string();
    let (mut context, user_opt) = request.get_session_context(&data).await;
    if let Some(user) = user_opt {
        sqlx::query!(
            "UPDATE public_calendar_links SET min_priority = $1, max_priority = $2 WHERE id = $3 AND user_id = $4",
            form.min_priority,
            form.max_priority,
            id,
            user.id
        )
        .execute(&data.conn)
        .await
        .map_err(TemplateOrDatabaseError::from)
        .or_internal_server_error("Failed to update min priority")?;

        context.insert(
            "export_links",
            &get_user_export_links(&data, user.id).await.map_err(|e| {
                TemplateOrDatabaseError::from(e.cause).internal_server_error(&e.context)
            })?,
        );
        let content = data
            .templates
            .render("components/export_link.html", &context)
            .map_err(TemplateOrDatabaseError::from)
            .or_internal_server_error("Failed to render template")?;
        return Ok(HttpResponse::Ok().body(content));
    }
    Ok(deauth(&request))
}

#[post("")]
async fn new_link(
    data: web::Data<AppState>,
    request: HttpRequest,
) -> Result<impl Responder, InternalServerError<TemplateOrDatabaseError>> {
    let (mut context, user_opt) = request.get_session_context(&data).await;
    if let Some(user) = user_opt {
        let new_id = Uuid::new_v4().to_string();
        sqlx::query!(
            "INSERT INTO public_calendar_links (id, user_id) VALUES ($1, $2)",
            new_id,
            user.id
        )
        .execute(&data.conn)
        .await
        .map_err(TemplateOrDatabaseError::from)
        .or_internal_server_error("Failed to insert new public link")?;

        context.insert(
            "export_links",
            &get_user_export_links(&data, user.id).await.map_err(|e| {
                TemplateOrDatabaseError::from(e.cause).internal_server_error(&e.context)
            })?,
        );
        let content = data
            .templates
            .render("components/export_link.html", &context)
            .map_err(TemplateOrDatabaseError::from)
            .or_internal_server_error("Failed to render template")?;
        return Ok(HttpResponse::Ok().body(content));
    }
    Ok(HttpResponse::Unauthorized().finish())
}

#[get("")]
async fn get_mine(
    data: web::Data<AppState>,
    request: HttpRequest,
) -> Result<impl Responder, InternalServerError<sqlx::Error>> {
    if let Some(user_id) = request.get_session_user(&data).await.map(|u| u.id) {
        let links = get_user_export_links(&data, user_id).await?;
        return Ok(HttpResponse::Ok().json(links));
    }
    Ok(HttpResponse::Unauthorized().finish())
}

pub fn routes() -> Scope {
    web::scope("/export")
        .service(new_link)
        .service(delete_link)
        .service(change_filters)
        .service(get_mine)
        //.service(get_local_calendar)
        .service(get_calendar)
}

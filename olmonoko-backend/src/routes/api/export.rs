use actix_web::{delete, get, patch, post, web, HttpRequest, HttpResponse, Responder, Scope};
use uuid::Uuid;

use crate::models::event::Priority;
use crate::models::public_link::{PublicLink, RawPublicLink};
use crate::routes::AppState;
use crate::utils::event_filters::EventFilter;
use crate::utils::events::get_visible_event_occurrences;
use crate::utils::request::{deauth, EnhancedRequest};
use crate::utils::user::get_user_export_links;

#[get("/{id}.ics")]
async fn get_calendar(data: web::Data<AppState>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner().to_string();
    tracing::info!("Fetching calendar for id {id}");
    let opt = sqlx::query_as!(
        RawPublicLink,
        "SELECT * FROM public_calendar_links WHERE id = $1",
        id
    )
    .fetch_optional(&data.conn)
    .await
    .expect("Failed to fetch user from the database")
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
        return HttpResponse::Ok().content_type("text/calendar").body(ics);
    } else {
        HttpResponse::NotFound().body("link not found")
    }
}

#[delete("/{id}.ics")]
async fn delete_link(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    request: HttpRequest,
) -> impl Responder {
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
        .expect("Failed to delete public link");
        HttpResponse::Ok().body("link deleted")
    } else {
        deauth(&request)
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
) -> impl Responder {
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
        .expect("Failed to update min priority");

        context.insert("export_links", &get_user_export_links(&data, user.id).await);
        let content = data
            .templates
            .render("components/export_link.html", &context)
            .unwrap();
        return HttpResponse::Ok().body(content);
    }
    deauth(&request)
}

#[post("")]
async fn new_link(data: web::Data<AppState>, request: HttpRequest) -> impl Responder {
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
        .expect("Failed to insert new public link");

        context.insert("export_links", &get_user_export_links(&data, user.id).await);
        let content = data
            .templates
            .render("components/export_link.html", &context)
            .unwrap();
        return HttpResponse::Ok().body(content);
    }
    HttpResponse::Unauthorized().finish()
}

#[get("")]
async fn get_mine(data: web::Data<AppState>, request: HttpRequest) -> impl Responder {
    if let Some(user_id) = request.get_session_user(&data).await.map(|u| u.id) {
        let links = get_user_export_links(&data, user_id).await;
        return HttpResponse::Ok().json(links);
    }
    HttpResponse::Unauthorized().finish()
}

pub fn routes() -> Scope {
    web::scope("/export")
        .service(new_link)
        .service(delete_link)
        .service(change_filters)
        .service(get_mine)
        .service(get_calendar)
}

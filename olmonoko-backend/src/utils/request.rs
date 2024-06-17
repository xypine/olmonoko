use super::{
    flash::{FlashMessage, FLASH_COOKIE_NAME},
    time::timestamp,
};
use crate::{
    models::{
        event::PRIORITY_OPTIONS,
        session::SessionRaw,
        user::{RawUser, User, UserPublic},
    },
    routes::AppState,
};
use actix_web::{web, HttpRequest, HttpResponse, HttpResponseBuilder};

pub(crate) const SESSION_COOKIE_NAME: &str = "session_id";

pub async fn get_user_from_request(data: &web::Data<AppState>, req: &HttpRequest) -> Option<User> {
    let session_cookie = req.cookie(SESSION_COOKIE_NAME);
    match session_cookie {
        None => None,
        Some(session_cookie) => {
            let session_id = session_cookie.value();
            let session = sqlx::query_as!(
                SessionRaw,
                "SELECT * FROM sessions WHERE id = $1",
                session_id
            )
            .fetch_optional(&data.conn)
            .await
            .unwrap();
            if let Some(session) = session {
                let expires_at = session.expires_at;
                if expires_at < timestamp() {
                    return None;
                }
                let user = sqlx::query_as!(
                    RawUser,
                    "SELECT * FROM users WHERE id = $1",
                    session.user_id
                )
                .fetch_optional(&data.conn)
                .await
                .map(|o| o.map(User::from))
                .unwrap();
                return user;
            }
            None
        }
    }
}

pub(crate) type SessionContext = (tera::Context, Option<UserPublic>);
pub(crate) async fn get_session_context(
    data: &web::Data<AppState>,
    request: &HttpRequest,
) -> SessionContext {
    let flash_message = request
        .cookie(FLASH_COOKIE_NAME)
        .map(|c| FlashMessage::from_cookie(&c));
    let user = get_user_from_request(data, request)
        .await
        .map(UserPublic::from);
    let path = request.path();
    let root_path = request.path().split('/').nth(1).unwrap_or("");
    let mut context = tera::Context::new();
    context.insert("site_url", &data.site_url);
    context.insert("path", &path);
    context.insert("root_path", &root_path);
    context.insert("version", &data.version);
    context.insert("flash", &flash_message);
    context.insert("user", &user);
    context.insert("event_priority_options", &PRIORITY_OPTIONS);
    (context, user)
}

pub(crate) trait EnhancedRequest {
    fn get_referer(&self) -> Option<&str>;
    fn get_session_id(&self) -> Option<String>;
    async fn get_session_user(&self, data: &web::Data<AppState>) -> Option<User>;
    async fn get_session_context(&self, data: &web::Data<AppState>) -> SessionContext;
}

impl EnhancedRequest for HttpRequest {
    fn get_referer(&self) -> Option<&str> {
        let headers = self.headers();
        headers.get("referer")?.to_str().ok()
    }
    fn get_session_id(&self) -> Option<String> {
        let session_cookie = self.cookie(SESSION_COOKIE_NAME)?;
        Some(session_cookie.value().to_string())
    }
    async fn get_session_user(&self, data: &web::Data<AppState>) -> Option<User> {
        get_user_from_request(data, self).await
    }
    async fn get_session_context(&self, data: &web::Data<AppState>) -> SessionContext {
        get_session_context(data, self).await
    }
}

pub fn deauth() -> HttpResponse {
    let mut removal_cookie = actix_web::cookie::Cookie::build(SESSION_COOKIE_NAME, "").finish();
    removal_cookie.make_removal();
    HttpResponse::Unauthorized().cookie(removal_cookie).finish()
}
pub fn redirect(location: &str) -> HttpResponseBuilder {
    tracing::info!("Redirecting to: {location}");
    let mut builder = HttpResponse::SeeOther();
    builder.insert_header((actix_web::http::header::LOCATION, location));
    builder
}
pub fn reload(req: &HttpRequest) -> HttpResponseBuilder {
    let location = req.get_referer().unwrap_or("/");
    redirect(location)
}

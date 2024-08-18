use actix_web::{
    body::BoxBody, web, HttpRequest, HttpResponse, HttpResponseBuilder, ResponseError,
};
use olmonoko_common::utils::{
    flash::{FlashMessage, FLASH_COOKIE_NAME},
    time::timestamp,
};
use olmonoko_common::{
    models::{
        event::PRIORITY_OPTIONS,
        session::SessionRaw,
        user::{RawUser, User, UserPublic},
    },
    AppState, APP_NAVIGATION_ENTRIES_ADMIN, APP_NAVIGATION_ENTRIES_LOGGEDIN,
    APP_NAVIGATION_ENTRIES_LOGGEDOUT, APP_NAVIGATION_ENTRIES_PUBLIC,
};

pub const SESSION_COOKIE_NAME: &str = "session_id";
pub const RESPONSE_TYPE_HEADER: &str = "HX-Request";

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
    let mut nav_entries = vec![];
    if let Some(user) = user.clone() {
        if user.admin {
            nav_entries.extend(APP_NAVIGATION_ENTRIES_ADMIN);
        }
        nav_entries.extend(APP_NAVIGATION_ENTRIES_LOGGEDIN);
    } else {
        nav_entries.extend(APP_NAVIGATION_ENTRIES_LOGGEDOUT);
    }
    nav_entries.extend(APP_NAVIGATION_ENTRIES_PUBLIC);
    for nav_entry in &mut nav_entries {
        if path.starts_with(nav_entry.path) {
            nav_entry.active = Some(true);
            break;
        } else {
            nav_entry.active = Some(false);
        }
    }
    nav_entries.sort_by_key(|e| e.position);
    context.insert("nav_entries", &nav_entries);
    (context, user)
}

#[allow(async_fn_in_trait)]
pub trait EnhancedRequest {
    fn get_referer(&self) -> Option<&str>;
    fn get_session_id(&self) -> Option<String>;
    async fn get_session_user(&self, data: &web::Data<AppState>) -> Option<User>;
    async fn get_session_context(&self, data: &web::Data<AppState>) -> SessionContext;
    fn is_frontend_request(&self) -> bool;
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
    fn is_frontend_request(&self) -> bool {
        if let Some(header) = self.headers().get(RESPONSE_TYPE_HEADER) {
            if header == "true" {
                return true;
            }
        }
        false
    }
}

pub fn deauth(req: &HttpRequest) -> HttpResponse {
    let mut removal_cookie = actix_web::cookie::Cookie::build(SESSION_COOKIE_NAME, "").finish();
    removal_cookie.make_removal();
    if req.is_frontend_request() {
        HttpResponse::Unauthorized().cookie(removal_cookie).finish()
    } else {
        HttpResponse::Unauthorized().cookie(removal_cookie).finish()
    }
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

pub trait IntoInternalServerError {
    type Err: std::fmt::Debug;
    fn internal_server_error(self, context: &str) -> InternalServerError<Self::Err>;
    fn internal_server_error_any(self, context: &str) -> AnyInternalServerError;
}
impl<T: std::fmt::Debug> IntoInternalServerError for T {
    type Err = T;
    fn internal_server_error(self, context: &str) -> InternalServerError<T> {
        InternalServerError::new(self, context.to_owned())
    }
    fn internal_server_error_any(self, context: &str) -> AnyInternalServerError {
        AnyInternalServerError::new(self, context.to_owned())
    }
}
pub trait OrInternalServerError {
    type Ok: std::fmt::Debug;
    type Err: std::fmt::Debug;
    fn or_internal_server_error(
        self,
        context: &str,
    ) -> Result<Self::Ok, InternalServerError<Self::Err>>;
    fn or_any_internal_server_error(
        self,
        context: &str,
    ) -> Result<Self::Ok, AnyInternalServerError>;
}
impl<O: std::fmt::Debug, E: std::fmt::Debug> OrInternalServerError for Result<O, E> {
    type Ok = O;
    type Err = E;
    fn or_internal_server_error(self, context: &str) -> Result<O, InternalServerError<E>> {
        self.map_err(|e| e.internal_server_error(context))
    }
    fn or_any_internal_server_error(self, context: &str) -> Result<O, AnyInternalServerError> {
        self.map_err(|e| e.internal_server_error_any(context))
    }
}
#[derive(Debug)]
pub struct InternalServerError<E: std::fmt::Debug> {
    pub cause: E,
    pub context: String,
}
impl<E: std::fmt::Debug> InternalServerError<E> {
    pub fn new(cause: E, context: String) -> Self {
        Self { cause, context }
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for InternalServerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{self:?}"))
    }
}
impl<E: std::fmt::Debug> ResponseError for InternalServerError<E> {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::InternalServerError().body("Internal Server Error")
    }
}

#[derive(Debug)]
pub struct AnyInternalServerError {
    pub cause: String,
}
impl AnyInternalServerError {
    pub fn new<E: std::fmt::Debug>(cause: E, context: String) -> Self {
        let imposter = InternalServerError { cause, context };
        let cause = format!("{imposter:?}");
        Self { cause }
    }
}
impl std::fmt::Display for AnyInternalServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.cause)
    }
}
impl ResponseError for AnyInternalServerError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::InternalServerError().body("Internal Server Error")
    }
}

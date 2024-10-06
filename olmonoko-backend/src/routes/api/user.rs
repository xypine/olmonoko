use actix_web::cookie::Cookie;
use actix_web::HttpRequest;
use actix_web::{delete, get, patch, post, web, HttpResponse, Responder, Scope};
use uuid::Uuid;

use crate::db_utils::request::{deauth, redirect, reload, EnhancedRequest, SESSION_COOKIE_NAME};
use olmonoko_common::models::session::{NewSession, SessionRaw};
use olmonoko_common::models::user::{NewUser, RawUser, UserForm, UserId, UserPublic};
use olmonoko_common::utils::flash::{FlashMessage, WithFlashMessage};
use olmonoko_common::AppState;

#[get("")]
async fn users(data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let user = req.get_session_user(&data).await;
    if let Some(user) = user {
        if !user.admin {
            return HttpResponse::Forbidden().body("You are not an admin");
        }
        let users: Vec<UserPublic> = sqlx::query_as!(RawUser, "SELECT * FROM users")
            .fetch_all(&data.conn)
            .await
            .expect("Failed to fetch users from the database")
            .iter()
            .map(|u| u.clone().into())
            .collect();
        return HttpResponse::Ok().json(users);
    }
    deauth(&req)
}

const MIN_PASSWORD_LENGTH: usize = 9; // We set this high to prevent brute force attacks as we don't have rate limiting yet
#[post("")]
async fn register(
    data: web::Data<AppState>,
    user: web::Form<UserForm>,
    req: HttpRequest,
) -> impl Responder {
    let user = user.into_inner();
    if user.password.len() < MIN_PASSWORD_LENGTH {
        return reload(&req)
            .with_flash_message(FlashMessage::error(
                format!(
                    "Password must be at least {} characters",
                    MIN_PASSWORD_LENGTH
                )
                .as_str(),
            ))
            .finish();
    }
    let mut active_user: NewUser = NewUser {
        email: user.email.clone(),
        password_hash: bcrypt::hash(&user.password, bcrypt::DEFAULT_COST).unwrap(),
        admin: false,
    };
    let unreliable_user_count = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(&data.conn)
        .await;
    let unreliable_user_count = if let Ok(count) = unreliable_user_count {
        count.expect("Failed to count users (2)")
    } else {
        tracing::error!("Failed to count users (1)");
        1
    };
    if unreliable_user_count == 0 {
        tracing::info!(user.email, "First user, promoting to admin");
        active_user.admin = true;
    }
    let result = crate::auth::create_unverified_user(&data, active_user).await;
    if result.is_ok() {
        return reload(&req)
            .with_flash_message(FlashMessage::info(
                "Check your email to verify your account",
            ))
            .finish();
    }
    tracing::error!("Failed to create user: {:?}", result);
    HttpResponse::InternalServerError().body("Failed to create user")
}

#[get("/verify/{secret}")]
async fn verify_user(data: web::Data<AppState>, secret: web::Path<String>) -> impl Responder {
    let secret = secret.into_inner();
    let result = crate::auth::verify_user(&data, &secret).await;
    match result {
        Ok(user) => {
            let five_days_from_now = chrono::Utc::now() + chrono::Duration::days(5);
            let new_session = NewSession {
                id: Uuid::new_v4().to_string(),
                user_id: user.id,
                expires_at: five_days_from_now.timestamp(),
            };
            let created = sqlx::query_as!(
                SessionRaw,
                "INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3) RETURNING *",
                new_session.id,
                new_session.user_id,
                new_session.expires_at
            )
            .fetch_one(&data.conn)
            .await
            .unwrap();
            let cookie = Cookie::build(SESSION_COOKIE_NAME, created.id.clone())
                .path("/")
                .secure(true)
                .http_only(true)
                .expires(None) // Change later
                .finish();
            return redirect("/")
                .with_flash_message(FlashMessage::info("Your account has been verified"))
                .cookie(cookie)
                .finish();
        }
        Err(e) => {
            tracing::error!("Failed to verify user: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to verify user")
        }
    }
}

#[delete("/{id}")]
async fn remove_user(
    data: web::Data<AppState>,
    req: HttpRequest,
    id: web::Path<UserId>,
) -> impl Responder {
    let (mut context, user, _key) = req.get_session_context(&data).await;
    if let Some(user) = user {
        if !user.admin {
            return HttpResponse::Forbidden().body("You are not an admin");
        }
        let id = id.into_inner();
        let user_count = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
            .fetch_one(&data.conn)
            .await
            .expect("Failed to count users (3)")
            .expect("Failed to count users (4)");
        if user_count == 1 {
            return HttpResponse::Forbidden().body("Cannot remove the last user");
        }
        let result = sqlx::query!("DELETE FROM users WHERE id = $1", id)
            .execute(&data.conn)
            .await;
        if result.is_ok() {
            let all_users = sqlx::query_as!(RawUser, "SELECT * FROM users")
                .fetch_all(&data.conn)
                .await
                .expect("Failed to get users");
            let all_users = all_users
                .into_iter()
                .map(UserPublic::from)
                .collect::<Vec<_>>();
            context.insert("users", &all_users);
            let content = data
                .templates
                .render("components/admin/user_list.html", &context)
                .unwrap();
            return HttpResponse::Ok().body(content);
        }
        tracing::error!("Failed to remove user: {:?}", result);
        return HttpResponse::InternalServerError().body("Failed to remove user");
    }
    deauth(&req)
}

#[post("/login")]
async fn login(
    data: web::Data<AppState>,
    user: web::Form<UserForm>,
    req: HttpRequest,
) -> impl Responder {
    let user_input = user.into_inner();
    if let Some(user) = sqlx::query!("SELECT * FROM users WHERE email = $1", user_input.email)
        .fetch_optional(&data.conn)
        .await
        .expect("Failed to fetch user for login")
    {
        if !bcrypt::verify(&user_input.password, &user.password_hash).unwrap() {
            tracing::warn!("Failed login attempt for {}", user.email);
            if req.is_frontend_request() {
                return reload(&req)
                    .with_flash_message(FlashMessage::error("Invalid email or password"))
                    .finish();
            }
            return HttpResponse::Forbidden().body("Invalid email or password");
        }
        let five_days_from_now = chrono::Utc::now() + chrono::Duration::days(5);
        let new_session = NewSession {
            id: Uuid::new_v4().to_string(),
            user_id: user.id,
            expires_at: five_days_from_now.timestamp(),
        };
        let created = sqlx::query_as!(
            SessionRaw,
            "INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3) RETURNING *",
            new_session.id,
            new_session.user_id,
            new_session.expires_at
        )
        .fetch_one(&data.conn)
        .await
        .expect("Failed to create session");
        let cookie = Cookie::build(SESSION_COOKIE_NAME, created.id.clone())
            .path("/")
            .secure(true)
            .http_only(true)
            .expires(None) // Change later
            .finish();
        if req.is_frontend_request() {
            return redirect("/").cookie(cookie).finish();
        }
        return HttpResponse::Ok().cookie(cookie).body(created.id.clone());
    }
    if req.is_frontend_request() {
        return reload(&req)
            .with_flash_message(FlashMessage::error("Invalid email or password"))
            .finish();
    }
    HttpResponse::Forbidden().body("Invalid email or password")
}

#[post("/logout")]
async fn logout(data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let session_cookie = req.cookie(SESSION_COOKIE_NAME).unwrap();
    let session_id = session_cookie.value();
    sqlx::query!("DELETE FROM sessions WHERE id = $1", session_id)
        .execute(&data.conn)
        .await
        .unwrap();
    let mut removal_cookie = Cookie::build(SESSION_COOKIE_NAME, "").finish();
    removal_cookie.make_removal();
    if req.is_frontend_request() {
        return reload(&req)
            .with_flash_message(FlashMessage::info("Goodbye!"))
            .cookie(removal_cookie)
            .finish();
    }
    return HttpResponse::Ok().cookie(removal_cookie).body("Goodbye!");
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeUserInterfaceTimezoneForm {
    interface_timezone: String,
}

#[patch("/timezone")]
async fn change_user_interface_timezone(
    data: web::Data<AppState>,
    req: HttpRequest,
    form: web::Form<ChangeUserInterfaceTimezoneForm>,
) -> impl Responder {
    let (mut context, user, _key) = req.get_session_context(&data).await;
    if let Some(mut user) = user {
        let timezone = form.interface_timezone.clone();
        let parsed_timezone: Option<chrono_tz::Tz> = timezone.parse().ok();
        if parsed_timezone.is_none() {
            return HttpResponse::BadRequest().body("Invalid timezone");
        }
        let _ = sqlx::query!(
            "UPDATE users SET interface_timezone = $1 WHERE id = $2",
            timezone,
            user.id
        )
        .execute(&data.conn)
        .await
        .expect("Failed to update user timezone");
        user.interface_timezone = timezone;
        context.insert("user", &user);

        let all_timezones = chrono_tz::TZ_VARIANTS
            .iter()
            .map(|tz| tz.name())
            .collect::<Vec<_>>();
        context.insert("timezones", &all_timezones);
        let component = data
            .templates
            .render("components/auth/change_timezone.html", &context)
            .unwrap();
        return HttpResponse::Ok().body(component);
    }
    deauth(&req)
}

#[get("/me")]
async fn me(data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Some(user) = req.get_session_user(&data).await {
        return HttpResponse::Ok().json(user);
    }
    deauth(&req)
}

pub fn routes() -> Scope {
    web::scope("/user")
        .service(users)
        .service(remove_user)
        .service(register)
        .service(verify_user)
        .service(login)
        .service(logout)
        .service(me)
        .service(change_user_interface_timezone)
}

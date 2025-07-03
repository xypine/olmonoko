use actix_web::HttpRequest;
use actix_web::{delete, post, web, HttpResponse, Responder, Scope};
use olmonoko_common::models::api_key::{ApiKey, ApiKeyForm, ApiKeyId, NewApiKey, RawApiKey};

use crate::db::errors::TemplateOrDatabaseError;
use crate::db::request::{AnyInternalServerError, EnhancedRequest, OrInternalServerError};
use olmonoko_common::AppState;

#[post("")]
async fn new(
    form: web::Form<ApiKeyForm>,
    data: web::Data<AppState>,
    request: HttpRequest,
) -> Result<impl Responder, AnyInternalServerError> {
    let (mut context, user_opt, key, _timer) = request.get_session_context(&data).await;

    if key.is_some() {
        return Ok(
            HttpResponse::BadRequest().body("API keys are not allowed to create new API keys")
        );
    }

    if let Some(user) = user_opt {
        let details = NewApiKey::try_from(form.into_inner())?;
        let result_raw = sqlx::query_as!(RawApiKey, r#"INSERT INTO api_keys (user_id, description, scopes, created_at) VALUES ($1, $2, $3, $4) RETURNING *"#, user.id, details.description, &details.scopes_pg, details.created_at).fetch_one(&data.conn).await.or_any_internal_server_error("Failed to insert new API key into db")?;
        let result = ApiKey::try_from(result_raw)?;
        if request.is_frontend_request() {
            let api_keys = sqlx::query_as!(
                RawApiKey,
                "SELECT * FROM api_keys WHERE user_id = $1",
                user.id
            )
            .fetch_all(&data.conn)
            .await
            .or_any_internal_server_error(
                "Failed to fetch api keys from db after inserting a new one",
            )?
            .into_iter()
            .map(|raw| ApiKey::try_from(raw).map(|key| ApiKeyForm::from(key)))
            .collect::<Result<Vec<_>, _>>()?;

            context.insert("api_keys", &api_keys);

            let content = data
                .templates
                .render("components/api_keys.html", &context)
                .map_err(TemplateOrDatabaseError::from)
                .or_any_internal_server_error("Failed to render template")?;
            return Ok(HttpResponse::Ok().body(content));
        } else {
            return Ok(HttpResponse::Ok().json(result));
        }
    }
    Ok(HttpResponse::Unauthorized().finish())
}

#[delete("/{id}")]
async fn revoke(
    data: web::Data<AppState>,
    path: web::Path<ApiKeyId>,
    request: HttpRequest,
) -> Result<impl Responder, AnyInternalServerError> {
    let (mut context, user_opt, key, _timer) = request.get_session_context(&data).await;

    if key.is_some() {
        return Ok(HttpResponse::BadRequest().body("API keys are not allowed to revoke API keys"));
    }

    if let Some(user) = user_opt {
        let key_id = path.into_inner();
        let result_raw = sqlx::query_as!(
            RawApiKey,
            r#"UPDATE api_keys SET revoked = TRUE WHERE id = $1 AND user_id = $2 RETURNING *"#,
            key_id,
            user.id
        )
        .fetch_one(&data.conn)
        .await
        .or_any_internal_server_error("Failed to revoke api key in db")?;
        let result = ApiKey::try_from(result_raw)?;
        if request.is_frontend_request() {
            let api_keys = sqlx::query_as!(
                RawApiKey,
                "SELECT * FROM api_keys WHERE user_id = $1",
                user.id
            )
            .fetch_all(&data.conn)
            .await
            .or_any_internal_server_error(
                "Failed to fetch api keys from db after inserting a new one",
            )?
            .into_iter()
            .map(|raw| ApiKey::try_from(raw).map(|key| ApiKeyForm::from(key)))
            .collect::<Result<Vec<_>, _>>()?;

            context.insert("api_keys", &api_keys);

            let content = data
                .templates
                .render("components/api_keys.html", &context)
                .map_err(TemplateOrDatabaseError::from)
                .or_any_internal_server_error("Failed to render template")?;
            return Ok(HttpResponse::Ok().body(content));
        } else {
            return Ok(HttpResponse::Ok().json(result));
        }
    }
    Ok(HttpResponse::Unauthorized().finish())
}

pub fn routes() -> Scope {
    web::scope("/key").service(new).service(revoke)
}

pub mod email;

use crate::{
    models::user::{NewUser, RawUser, User},
    routes::AppState,
};

#[derive(Debug, thiserror::Error)]
pub enum UnverifiedUserCreationError {
    #[error("Database error: {0}")]
    DbErr(#[from] sqlx::Error),
    #[error("Failed to send verification email")]
    EmailErr(#[from] email::Error),
}

pub async fn create_unverified_user(
    data: &AppState,
    user: NewUser,
) -> Result<(), UnverifiedUserCreationError> {
    let secret = uuid::Uuid::new_v4().to_string();
    sqlx::query!(
        r#"
        INSERT INTO unverified_users (email, password_hash, admin, secret)
        VALUES (?, ?, ?, ?)
        ON CONFLICT (email) DO UPDATE SET secret = EXCLUDED.secret
        "#,
        user.email,
        user.password_hash,
        user.admin,
        secret
    )
    .execute(&data.conn)
    .await?;
    email::send_verification_email(data, &user.email, &secret).await?;
    Ok(())
}

pub async fn verify_user(data: &AppState, secret: &str) -> Result<User, sqlx::Error> {
    let user = sqlx::query!(
        r#"
        DELETE
        FROM unverified_users
        WHERE secret = ?
        RETURNING *
        "#,
        secret
    )
    .fetch_one(&data.conn)
    .await?;
    let user = sqlx::query_as!(
        RawUser,
        r#"
        INSERT INTO users (email, password_hash, admin)
        VALUES (?, ?, ?)
        RETURNING *
        "#,
        user.email,
        user.password_hash,
        user.admin
    )
    .fetch_one(&data.conn)
    .await
    .map(User::from)?;
    Ok(user)
}

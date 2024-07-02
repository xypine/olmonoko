use crate::utils::time::from_timestamp;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawUser {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub admin: bool,
    pub created_at: i64,
    pub interface_timezone: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub interface_timezone: String,
    #[serde(skip)]
    pub interface_timezone_parsed: chrono_tz::Tz,
}
impl From<RawUser> for User {
    fn from(raw: RawUser) -> Self {
        Self {
            id: raw.id,
            email: raw.email,
            password_hash: raw.password_hash,
            admin: raw.admin,
            created_at: from_timestamp(raw.created_at),
            interface_timezone: raw.interface_timezone.clone(),
            interface_timezone_parsed: raw
                .interface_timezone
                .parse()
                .expect("Failed to parse timezone"),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub admin: bool,
}
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct UserForm {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct UserPublic {
    pub id: i64,
    pub email: String,
    pub admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub interface_timezone: String,
    #[serde(skip)]
    pub interface_timezone_parsed: chrono_tz::Tz,
}
impl From<User> for UserPublic {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            admin: user.admin,
            created_at: user.created_at,
            interface_timezone: user.interface_timezone,
            interface_timezone_parsed: user.interface_timezone_parsed,
        }
    }
}
impl From<RawUser> for UserPublic {
    fn from(raw: RawUser) -> Self {
        User::from(raw).into()
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct UnverifiedUser {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub created_at: i64,
    pub admin: bool,
    pub secret: String,
}

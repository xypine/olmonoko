use uuid::Uuid;

use crate::utils::time::from_timestamp;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionRaw {
    pub id: String,
    pub user_id: i64,
    pub expires_at: i64,
    pub created_at: i64,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: i64,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
impl From<SessionRaw> for Session {
    fn from(raw: SessionRaw) -> Self {
        Self {
            id: Uuid::parse_str(&raw.id).unwrap(),
            user_id: raw.user_id,
            expires_at: from_timestamp(raw.expires_at),
            created_at: from_timestamp(raw.created_at),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct NewSession {
    pub id: String,
    pub user_id: i64,
    pub expires_at: i64,
}

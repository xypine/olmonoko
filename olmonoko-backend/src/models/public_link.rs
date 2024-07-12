use uuid::Uuid;

use crate::{routes::get_site_url, utils::time::from_timestamp};

use super::{event::Priority, user::UserId};

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawPublicLink {
    pub id: String,
    pub user_id: UserId,
    pub created_at: i64,
    pub min_priority: Option<Priority>,
    pub max_priority: Option<Priority>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PublicLink {
    pub id: Uuid,
    pub user_id: UserId,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub min_priority: Option<Priority>,
    pub max_priority: Option<Priority>,
    pub url: String,
}
impl From<RawPublicLink> for PublicLink {
    fn from(raw: RawPublicLink) -> Self {
        let site_url = get_site_url();
        Self {
            id: Uuid::parse_str(&raw.id).unwrap(),
            user_id: raw.user_id,
            created_at: from_timestamp(raw.created_at),
            min_priority: raw.min_priority,
            max_priority: raw.max_priority,
            url: format!("{}/api/export/{}.ics", site_url, raw.id),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct NewPublicLink {
    pub user_id: i64,
    pub created_at: i64,
}

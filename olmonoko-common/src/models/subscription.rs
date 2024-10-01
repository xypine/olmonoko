use chrono::{DateTime, Utc};

use crate::utils::time::from_timestamp;

use super::{event::Priority, ics_source::IcsSourceId, user::UserId};

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawSubscription {
    pub user_id: UserId,
    pub ics_source_id: IcsSourceId,
    pub priority: Priority,
    pub import_template: Option<String>,

    pub imported_at: Option<i64>,
    pub imported_hash: Option<String>,
    pub imported_version: Option<String>,
}

impl From<RawSubscription> for Subscription {
    fn from(raw: RawSubscription) -> Self {
        Self {
            user_id: raw.user_id,
            ics_source_id: raw.ics_source_id,
            priority: raw.priority,
            import_template: raw.import_template,
            imported_at: raw.imported_at.map(from_timestamp),
            imported_hash: raw.imported_hash,
            imported_version: raw.imported_version,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Subscription {
    pub user_id: UserId,
    pub ics_source_id: IcsSourceId,
    pub priority: Priority,
    pub import_template: Option<String>,

    pub imported_at: Option<DateTime<Utc>>,
    pub imported_hash: Option<String>,
    pub imported_version: Option<String>,
}

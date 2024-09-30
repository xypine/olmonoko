use std::collections::BTreeSet;

use chrono::Utc;
use uuid::Uuid;

use crate::utils::time::from_timestamp;

use super::user::UserId;

pub type ApiKeyId = Uuid;

const AUTHSCOPE_RF_UPCOMING_EVENTS: &str = "olmonoko:r:feature:upcoming_events";
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum AuthScope {
    ReadFeatureUpcomingEvents,
}
impl TryFrom<String> for AuthScope {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            AUTHSCOPE_RF_UPCOMING_EVENTS => return Ok(Self::ReadFeatureUpcomingEvents),
            _ => {}
        }
        Err("Not a valid AuthScope")
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawApiKey {
    pub id: ApiKeyId,
    pub user_id: UserId,
    pub description: String,
    pub scopes: Vec<String>,
    pub revoked: bool,

    pub created_at: i64,
    pub updated_at: i64,
}

impl TryFrom<RawApiKey> for ApiKey {
    type Error = &'static str;

    fn try_from(raw: RawApiKey) -> Result<Self, Self::Error> {
        let scopes = raw
            .scopes
            .into_iter()
            .map(|scope| AuthScope::try_from(scope).unwrap())
            .collect();
        Ok(Self {
            id: raw.id,
            user_id: raw.user_id,
            description: raw.description,
            scopes,
            revoked: raw.revoked,
            created_at: from_timestamp(raw.created_at),
            updated_at: from_timestamp(raw.updated_at),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub user_id: UserId,
    pub description: String,
    pub scopes: BTreeSet<AuthScope>,
    pub revoked: bool,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

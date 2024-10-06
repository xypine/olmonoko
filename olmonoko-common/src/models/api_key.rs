use std::collections::BTreeSet;
use std::fmt::Display;

use chrono::Utc;
use uuid::Uuid;

use serde_with::As;
use serde_with::NoneAsEmptyString;

use crate::utils::time::from_timestamp;
use crate::utils::time::timestamp;

use super::user::UserId;

pub type ApiKeyId = Uuid;

const AUTHSCOPE_FEATURE_UPCOMING_EVENTS: &str = "upcoming_events";
const AUTHSCOPE_RESOURCE_ATTENDANCE_READ: &str = "attendance:r";
const AUTHSCOPE_RESOURCE_ATTENDANCE_WRITE: &str = "attendance:w";
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum AuthScope {
    UpcomingEventsFeature,
    AttendanceRead,
    AttendanceWrite,
}
impl TryFrom<&str> for AuthScope {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            AUTHSCOPE_FEATURE_UPCOMING_EVENTS => Ok(Self::UpcomingEventsFeature),
            AUTHSCOPE_RESOURCE_ATTENDANCE_READ => Ok(Self::AttendanceRead),
            AUTHSCOPE_RESOURCE_ATTENDANCE_WRITE => Ok(Self::AttendanceWrite),
            _ => Err("Not a valid AuthScope"),
        }
    }
}
impl Display for AuthScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let as_str = match self {
            AuthScope::UpcomingEventsFeature => AUTHSCOPE_FEATURE_UPCOMING_EVENTS,
            AuthScope::AttendanceRead => AUTHSCOPE_RESOURCE_ATTENDANCE_READ,
            AuthScope::AttendanceWrite => AUTHSCOPE_RESOURCE_ATTENDANCE_WRITE,
        };
        f.write_str(as_str)
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
            .map(|scope| {
                AuthScope::try_from(scope.as_str())
                    .expect("parsing API scope returned from database")
            })
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewApiKey {
    pub description: String,
    pub scopes: BTreeSet<AuthScope>,
    pub scopes_pg: Vec<String>,
    pub created_at: i64,
}

impl TryFrom<ApiKeyForm> for NewApiKey {
    type Error = &'static str;

    fn try_from(form: ApiKeyForm) -> Result<Self, Self::Error> {
        let mut scopes = BTreeSet::new();
        for scope_str in form.scopes.split(',') {
            let trimmed = scope_str.trim();
            if !trimmed.is_empty() {
                let scope = AuthScope::try_from(trimmed)?;
                scopes.insert(scope);
            }
        }
        let scopes_pg: Vec<_> = scopes.iter().map(AuthScope::to_string).collect();

        Ok(Self {
            description: form.description,
            created_at: timestamp(),
            scopes,
            scopes_pg,
        })
    }
}

impl From<ApiKey> for ApiKeyForm {
    fn from(key: ApiKey) -> Self {
        let scopes_vec: Vec<_> = key.scopes.iter().map(AuthScope::to_string).collect();
        let scopes = scopes_vec.join(", ");
        let created_at = Some(key.created_at.to_string());
        let updated_at = Some(key.updated_at.to_string());
        let revoked = Some(key.revoked);
        Self {
            id: Some(key.id),
            description: key.description,
            scopes,
            created_at,
            updated_at,
            revoked,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ApiKeyForm {
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub id: Option<ApiKeyId>,
    pub description: String,
    pub scopes: String,

    #[serde(skip_deserializing)]
    pub created_at: Option<String>,
    #[serde(skip_deserializing)]
    pub updated_at: Option<String>,
    #[serde(skip_deserializing)]
    pub revoked: Option<bool>,
}

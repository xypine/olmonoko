use chrono::Utc;
use uuid::Uuid;

use serde_with::As;
use serde_with::NoneAsEmptyString;

use crate::utils::time::from_timestamp;
use crate::utils::time::timestamp;
use super::event::local::LocalEventId;
use super::user::UserId;

pub type TimerId = Uuid;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawTimer {
    pub id: TimerId,
    pub user_id: UserId,
    pub summary: Option<String>,
    pub details: Option<String>,
    pub location: Option<String>,

    pub template: LocalEventId,

    pub created_at: i64,
}

impl From<RawTimer> for Timer {
    fn from(raw: RawTimer) -> Self {
        Timer {
            id: raw.id,
            user_id: raw.user_id,
            summary: raw.summary,
            details: raw.details,
            location: raw.location,

            template: raw.template,

            created_at: from_timestamp(raw.created_at)
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Timer {
    pub id: TimerId,
    pub user_id: UserId,
    pub summary: Option<String>,
    pub details: Option<String>,
    pub location: Option<String>,

    pub template: LocalEventId,

    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewTimer {
    pub summary: Option<String>,
    pub details: Option<String>,
    pub location: Option<String>,

    pub template: LocalEventId,
    pub created_at: i64
}

impl From<TimerForm> for NewTimer {

    fn from(form: TimerForm) -> Self {
        Self {
            summary: form.summary,
            details: form.details,
            location: form.location,

            template: form.template,
            created_at: timestamp()
        }
    }
}

impl From<Timer> for TimerForm {
    fn from(timer: Timer) -> Self {
        let created_at = Some(timer.created_at.to_string());
        Self {
            id: Some(timer.id),

            summary: timer.summary,
            details: timer.details,
            location: timer.location,

            template: timer.template,

            created_at
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct TimerForm {
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub id: Option<TimerId>,

    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub summary: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub details: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub location: Option<String>,

    pub template: LocalEventId,

    #[serde(skip_deserializing)]
    pub created_at: Option<String>,
}

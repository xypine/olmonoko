use chrono::Utc;

use crate::utils::time::from_timestamp;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawAttendance {
    #[serde(skip)]
    pub id: i32,
    pub user_id: i32,

    pub local_event_id: Option<i32>,
    pub remote_event_id: Option<i32>,

    pub planned: bool,

    pub actual: bool,

    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Attendance {
    pub planned: bool,
    pub actual: bool,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewAttendance {
    pub user_id: i32,
    pub local_event_id: LocalEventId,

    pub planned: bool,
    pub actual: bool,
}

impl From<RawAttendance> for Attendance {
    fn from(raw: RawAttendance) -> Self {
        Self {
            planned: raw.planned,
            actual: raw.actual,
            created_at: from_timestamp(raw.created_at / 1000),
            updated_at: from_timestamp(raw.updated_at / 1000),
        }
    }
}

use crate::models::ics_source::deserialize_checkbox;
use crate::models::ics_source::serialize_checkbox;

use super::event::local::LocalEventId;
use super::user::UserId;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AttendanceForm {
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub attend_plan: bool,
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub attend_actual: bool,
}

impl From<(AttendanceForm, UserId, LocalEventId)> for NewAttendance {
    fn from((form, user_id, local_event_id): (AttendanceForm, UserId, LocalEventId)) -> Self {
        Self {
            user_id,
            local_event_id,
            planned: form.attend_plan,
            actual: form.attend_actual,
        }
    }
}

impl From<Attendance> for AttendanceForm {
    fn from(f: Attendance) -> Self {
        Self {
            attend_plan: f.planned,
            attend_actual: f.actual,
        }
    }
}
impl From<NewAttendance> for AttendanceForm {
    fn from(value: NewAttendance) -> Self {
        AttendanceForm {
            attend_actual: value.actual,
            attend_plan: value.planned,
        }
    }
}

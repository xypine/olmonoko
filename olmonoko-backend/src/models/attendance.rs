use chrono::Utc;

use crate::utils::time::from_timestamp;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawAttendance {
    pub user_id: i64,

    pub local_event_id: Option<i64>,
    pub remote_event_id: Option<i64>,

    pub planned: bool,
    pub planned_starts_at: Option<i64>,
    pub planned_duration: Option<i64>,

    pub actual: bool,
    pub actual_starts_at: Option<i64>,
    pub actual_duration: Option<i64>,

    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttendanceDetails {
    pub starts_at: Option<i64>,
    pub duration: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AutoAttendanceDetails {
    pub starts_at: Option<i64>,
    pub duration: Option<i64>,

    pub start: chrono::DateTime<Utc>,
    pub end: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AttendanceEvent {
    Local(i64),
    Remote(i64),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Attendance<D> {
    pub user_id: i64,

    pub event_id: AttendanceEvent,

    pub planned: Option<D>,
    pub actual: Option<D>,

    pub created_at: chrono::DateTime<Utc>,
}

impl TryFrom<RawAttendance> for Attendance<AttendanceDetails> {
    type Error = &'static str;
    fn try_from(raw: RawAttendance) -> Result<Self, Self::Error> {
        let event_id = match (raw.local_event_id, raw.remote_event_id) {
            (None, None) => Err("Raw attendance missing any remote id"),
            (None, Some(remote_id)) => Ok(AttendanceEvent::Remote(remote_id)),
            (Some(local_id), None) => Ok(AttendanceEvent::Local(local_id)),
            (Some(_), Some(_)) => Err("Raw attendance has two ids"),
        }?;

        let planned = if raw.planned {
            Some(AttendanceDetails {
                starts_at: raw.planned_starts_at,
                duration: raw.planned_duration,
            })
        } else {
            None
        };

        let actual = if raw.actual {
            Some(AttendanceDetails {
                starts_at: raw.actual_starts_at,
                duration: raw.actual_duration,
            })
        } else {
            None
        };

        Ok(Self {
            user_id: raw.user_id,
            event_id,
            planned,
            actual,
            created_at: from_timestamp(raw.created_at),
        })
    }
}

impl TryFrom<(RawAttendance, i64, i64)> for Attendance<AutoAttendanceDetails> {
    type Error = &'static str;
    fn try_from(
        (raw, starts_at, duration): (RawAttendance, i64, i64),
    ) -> Result<Self, Self::Error> {
        let event_id = match (raw.local_event_id, raw.remote_event_id) {
            (None, None) => Err("Raw attendance missing any remote id"),
            (None, Some(remote_id)) => Ok(AttendanceEvent::Remote(remote_id)),
            (Some(local_id), None) => Ok(AttendanceEvent::Local(local_id)),
            (Some(_), Some(_)) => Err("Raw attendance has two ids"),
        }?;

        let planned = if raw.planned {
            let start = raw.planned_starts_at.unwrap_or(starts_at);
            let end = start + raw.planned_duration.unwrap_or(duration);
            Some(AutoAttendanceDetails {
                starts_at: raw.planned_starts_at,
                duration: raw.planned_duration,

                start: from_timestamp(start),
                end: from_timestamp(end),
            })
        } else {
            None
        };

        let actual = if raw.actual {
            let start = raw.actual_starts_at.unwrap_or(starts_at);
            let end = start + raw.actual_duration.unwrap_or(duration);
            Some(AutoAttendanceDetails {
                starts_at: raw.actual_starts_at,
                duration: raw.actual_duration,

                start: from_timestamp(start),
                end: from_timestamp(end),
            })
        } else {
            None
        };

        Ok(Self {
            user_id: raw.user_id,
            event_id,
            planned,
            actual,
            created_at: from_timestamp(raw.created_at),
        })
    }
}

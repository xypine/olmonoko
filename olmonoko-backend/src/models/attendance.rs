use chrono::Utc;
use sqlx::Executor;
use sqlx::Postgres;

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
pub enum AttendanceEvent {
    Local(i32),
    Remote(i32),
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
    pub event_id: AttendanceEvent,

    pub planned: bool,
    pub actual: bool,
}
impl NewAttendance {
    pub async fn write<C>(&self, conn: &mut C) -> Result<Option<Attendance>, sqlx::Error>
    where
        for<'e> &'e mut C: Executor<'e, Database = Postgres>,
    {
        let (local_event_id, remote_event_id) = match self.event_id {
            AttendanceEvent::Local(id) => (Some(id), None),
            AttendanceEvent::Remote(id) => (None, Some(id)),
        };
        if self.planned || self.actual {
            let raw = sqlx::query_as!(
                RawAttendance,
                r#"
                INSERT INTO attendance
                    ( user_id, local_event_id, remote_event_id, planned, actual )
                VALUES
                    ( $1, $2, $3, $4, $5 )
                ON CONFLICT(user_id, coalesce(local_event_id, -1), coalesce(remote_event_id, -1)) DO UPDATE SET
                    planned = excluded.planned,
                    actual = excluded.actual,
                    updated_at = EXTRACT(EPOCH FROM NOW())*1000
                RETURNING *
            "#,
                self.user_id,
                local_event_id,
                remote_event_id,
                self.planned,
                self.actual,
            )
            .fetch_one(&mut *conn)
            .await?;

            return Ok(Some(Attendance::from(raw)));
        } else {
            sqlx::query!("DELETE FROM attendance WHERE user_id = $1 AND (local_event_id = $2 OR $2 IS NULL) AND (remote_event_id = $3 OR $3 IS NULL)", self.user_id, local_event_id, remote_event_id)
            .execute(&mut *conn)
            .await?;
        }

        Ok(None)
    }
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

impl From<(AttendanceForm, UserId, AttendanceEvent)> for NewAttendance {
    fn from((form, user_id, event_id): (AttendanceForm, UserId, AttendanceEvent)) -> Self {
        Self {
            user_id,
            event_id,
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

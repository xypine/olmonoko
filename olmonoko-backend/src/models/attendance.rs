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
    pub planned_starts_at: Option<i64>,
    pub planned_duration: Option<i32>,

    pub actual: bool,
    pub actual_starts_at: Option<i64>,
    pub actual_duration: Option<i32>,

    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttendanceDetails {
    pub starts_at: Option<i64>,
    pub duration: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtraAttendanceDetails {
    pub starts_at: Option<i64>,
    pub duration: Option<i32>,

    pub start: chrono::DateTime<Utc>,
    pub end: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AttendanceEvent {
    Local(i32),
    Remote(i32),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Attendance {
    pub planned: Option<ExtraAttendanceDetails>,
    pub actual: Option<ExtraAttendanceDetails>,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewAttendance {
    pub user_id: i32,
    pub event_id: AttendanceEvent,

    pub planned: Option<AttendanceDetails>,
    pub actual: Option<AttendanceDetails>,
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
        let (planned, planned_starts_at, planned_duration) = match &self.planned {
            Some(p) => (true, p.starts_at, p.duration),
            _ => (false, None, None),
        };
        let (actual, actual_starts_at, actual_duration) = match &self.actual {
            Some(a) => (true, a.starts_at, a.duration),
            _ => (false, None, None),
        };
        if planned || actual {
            let a = sqlx::query_as!(
                RawAttendance,
                r#"
                INSERT INTO attendance
                    ( user_id, local_event_id, remote_event_id, planned, planned_starts_at, planned_duration, actual, actual_starts_at, actual_duration )
                VALUES
                    ( $1, $2, $3, $4, $5, $6, $7, $8, $9 )
                ON CONFLICT(user_id, local_event_id, remote_event_id) DO UPDATE SET
                    planned = excluded.planned,
                    planned_starts_at = excluded.planned_starts_at,
                    planned_duration = excluded.planned_duration,
                    actual = excluded.actual,
                    actual_starts_at = excluded.actual_starts_at,
                    actual_duration = excluded.actual_duration,
                    updated_at = EXTRACT(EPOCH FROM NOW())*1000
                RETURNING *
            "#,
                self.user_id,
                local_event_id,
                remote_event_id,
                planned,
                planned_starts_at,
                planned_duration,
                actual,
                actual_starts_at,
                actual_duration
            )
            .fetch_one(&mut *conn)
            .await?;
        } else {
            sqlx::query!("DELETE FROM attendance WHERE user_id = $1 AND (local_event_id = $2 OR $2 IS NULL) AND (remote_event_id = $3 OR $3 IS NULL)", self.user_id, local_event_id, remote_event_id)
            .execute(&mut *conn)
            .await?;
        }

        Ok(None)
    }
}

impl From<(RawAttendance, i64, Option<i32>)> for Attendance {
    fn from((raw, starts_at, event_duration): (RawAttendance, i64, Option<i32>)) -> Self {
        let calc_end = |start: i64, duration: Option<i32>| match (event_duration, duration) {
            (_, Some(duration)) => Some(start + duration as i64),
            (Some(event_duration), None) => Some(start + event_duration as i64),
            (None, None) => None,
        };

        let planned = if raw.planned {
            let start = raw.planned_starts_at.unwrap_or(starts_at);
            let end = calc_end(start, raw.planned_duration).map(from_timestamp);
            Some(ExtraAttendanceDetails {
                starts_at: raw.planned_starts_at,
                duration: raw.planned_duration,

                start: from_timestamp(start),
                end,
            })
        } else {
            None
        };

        let actual = if raw.actual {
            let start = raw.actual_starts_at.unwrap_or(starts_at);
            let end = calc_end(start, raw.planned_duration).map(from_timestamp);
            Some(ExtraAttendanceDetails {
                starts_at: raw.actual_starts_at,
                duration: raw.actual_duration,

                start: from_timestamp(start),
                end,
            })
        } else {
            None
        };

        Self {
            planned,
            actual,
            created_at: from_timestamp(raw.created_at),
            updated_at: from_timestamp(raw.updated_at),
        }
    }
}

use crate::models::ics_source::deserialize_checkbox;
use crate::models::ics_source::serialize_checkbox;
use serde_with::As;
use serde_with::NoneAsEmptyString;

use super::user::UserPublic;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AttendanceForm {
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub attend_plan: bool,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub attend_plan_start: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub attend_plan_end: Option<String>,
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub attend_actual: bool,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub attend_actual_start: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub attend_actual_end: Option<String>,
}

pub type AttendanceFormWithUserEventTz<'a> =
    (AttendanceForm, &'a UserPublic, AttendanceEvent, i64, i8);
impl<'a> TryFrom<AttendanceFormWithUserEventTz<'a>> for NewAttendance {
    type Error = &'static str;
    fn try_from(
        (form, user, event_id, event_starts_at, tz_offset): AttendanceFormWithUserEventTz,
    ) -> Result<Self, Self::Error> {
        let parse_time = |f: String| crate::utils::time::from_form(&f, tz_offset).timestamp();
        let calc_duration = |start: Option<i64>, end: i64| end - start.unwrap_or(event_starts_at);

        let planned = if form.attend_plan {
            let starts_at = form.attend_plan_start.map(parse_time);
            let end = form.attend_plan_end.map(parse_time);
            let duration = end.map(|e| calc_duration(starts_at, e) as i32);
            Some(AttendanceDetails {
                starts_at,
                duration,
            })
        } else {
            None
        };

        let actual = if form.attend_actual {
            let starts_at = form.attend_actual_start.map(parse_time);
            let end = form.attend_actual_end.map(parse_time);
            let duration = end.map(|e| calc_duration(starts_at, e) as i32);
            Some(AttendanceDetails {
                starts_at,
                duration,
            })
        } else {
            None
        };

        Ok(Self {
            user_id: user.id,
            event_id,
            planned,
            actual,
        })
    }
}

impl From<Attendance> for AttendanceForm {
    fn from(f: Attendance) -> Self {
        use crate::utils::time::to_form;
        let (attend_plan, plan_start, plan_end) = match f.planned {
            Some(p) => (true, p.starts_at.map(|_| p.start), p.end),
            None => (false, None, None),
        };
        let (attend_actual, actual_start, actual_end) = match f.actual {
            Some(a) => (true, a.starts_at.map(|_| a.start), a.end),
            None => (false, None, None),
        };
        Self {
            attend_plan,
            attend_plan_start: plan_start.and_then(to_form),
            attend_plan_end: plan_end.and_then(to_form),
            attend_actual,
            attend_actual_start: actual_start.and_then(to_form),
            attend_actual_end: actual_end.and_then(to_form),
        }
    }
}
impl From<NewAttendance> for AttendanceForm {
    fn from(value: NewAttendance) -> Self {
        todo!()
    }
}

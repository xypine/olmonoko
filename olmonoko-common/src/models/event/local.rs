use std::vec;

use chrono::Utc;

use serde_with::As;
use serde_with::NoneAsEmptyString;

use crate::models::attendance::Attendance;
use crate::models::attendance::AttendanceForm;
use crate::models::bills::AutoDescription;
use crate::models::bills::Bill;
use crate::models::bills::RawBill;
use crate::models::user::UserId;
use crate::models::user::UserPublic;
use crate::utils::time::from_timestamp;

pub type LocalEventId = i32;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawLocalEvent {
    pub id: LocalEventId,
    pub user_id: UserId,
    pub created_at: i64,
    pub updated_at: i64,
    pub priority: Option<Priority>,
    // Event data
    pub rrule: Option<String>,
    pub starts_at: i64,
    pub all_day: bool,
    pub duration: Option<i32>,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
impl EventLike for RawLocalEvent {
    fn id(&self) -> EventId {
        self.id
    }
    fn source(&self) -> EventSource {
        EventSource::Local(SourceLocal {
            user_id: self.user_id,
        })
    }
    fn priority(&self) -> Option<Priority> {
        self.priority
    }
    fn tags(&self) -> Vec<String> {
        vec![] // FIX: This is ridiculous, the trait is getting unusable for raw events
    }
    fn all_day(&self) -> bool {
        self.all_day
    }
    fn starts_at(&self) -> Vec<i64> {
        vec![self.starts_at]
    }
    fn duration(&self) -> Option<i32> {
        self.duration
    }
    fn summary(&self) -> &str {
        &self.summary
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalEvent {
    pub id: LocalEventId,
    pub user_id: UserId,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub priority: Option<Priority>,
    pub tags: Vec<String>,
    pub attendance: Option<Attendance>,
    // Event data
    pub rrule: Option<String>,
    pub starts_at: chrono::DateTime<Utc>,
    pub all_day: bool,
    pub duration: Option<i32>,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
    // Attachments
    pub bill: Option<Bill>,
}
impl From<RawLocalEvent> for LocalEvent {
    fn from(raw: RawLocalEvent) -> Self {
        Self {
            id: raw.id,
            user_id: raw.user_id,
            created_at: from_timestamp(raw.created_at),
            updated_at: from_timestamp(raw.updated_at),
            priority: raw.priority,
            tags: vec![],
            attendance: None,
            rrule: raw.rrule,
            starts_at: from_timestamp(raw.starts_at),
            all_day: raw.all_day,
            duration: raw.duration,
            summary: raw.summary,
            description: raw.description,
            location: raw.location,
            uid: raw.uid,
            bill: None,
        }
    }
}
impl From<(RawLocalEvent, Vec<String>, Option<Attendance>)> for LocalEvent {
    fn from((raw, tags, attendance): (RawLocalEvent, Vec<String>, Option<Attendance>)) -> Self {
        Self {
            id: raw.id,
            user_id: raw.user_id,
            created_at: from_timestamp(raw.created_at),
            updated_at: from_timestamp(raw.updated_at),
            priority: raw.priority,
            tags,
            attendance,
            rrule: raw.rrule,
            starts_at: from_timestamp(raw.starts_at),
            all_day: raw.all_day,
            duration: raw.duration,
            summary: raw.summary,
            description: raw.description,
            location: raw.location,
            uid: raw.uid,
            bill: None,
        }
    }
}
impl From<(RawLocalEvent, &str, Option<Attendance>)> for LocalEvent {
    fn from((raw, tags_concat, attendance): (RawLocalEvent, &str, Option<Attendance>)) -> Self {
        let tags: Vec<_> = tags_concat.split(',').map(|s| s.to_string()).collect();
        Self::from((raw, tags, attendance))
    }
}
impl From<(RawLocalEvent, &str)> for LocalEvent {
    fn from((raw, tags_concat): (RawLocalEvent, &str)) -> Self {
        Self::from((raw, tags_concat, None))
    }
}

impl
    From<(
        RawLocalEvent,
        Option<RawBill>,
        bool,
        Vec<String>,
        Option<Attendance>,
    )> for LocalEvent
{
    fn from(
        (raw, bill, autodescription, tags, attendance): (
            RawLocalEvent,
            Option<RawBill>,
            bool,
            Vec<String>,
            Option<Attendance>,
        ),
    ) -> Self {
        let bill = bill.map(Bill::from);
        let description = if autodescription {
            if raw.description.is_some() || bill.is_some() {
                let mut parts = vec![];
                if let Some(desc) = &raw.description {
                    parts.push(desc.clone());
                }
                if let Some(bill) = bill.as_ref() {
                    parts.push(bill.generate_description(Some(&raw)));
                }
                Some(parts.join("\n\n"))
            } else {
                None
            }
        } else {
            raw.description
        };
        Self {
            id: raw.id,
            user_id: raw.user_id,
            created_at: from_timestamp(raw.created_at),
            updated_at: from_timestamp(raw.updated_at),
            priority: raw.priority,
            tags,
            attendance,
            starts_at: from_timestamp(raw.starts_at),
            rrule: raw.rrule,
            all_day: raw.all_day,
            duration: raw.duration,
            summary: raw.summary,
            description,
            location: raw.location,
            uid: raw.uid,
            bill: bill.map(Bill::from),
        }
    }
}
impl
    From<(
        RawLocalEvent,
        Option<RawBill>,
        bool,
        &str,
        Option<Attendance>,
    )> for LocalEvent
{
    fn from(
        (raw, bill, autodescription, tags_concat, attendance): (
            RawLocalEvent,
            Option<RawBill>,
            bool,
            &str,
            Option<Attendance>,
        ),
    ) -> Self {
        let tags: Vec<_> = tags_concat.split(',').map(|s| s.to_string()).collect();
        Self::from((raw, bill, autodescription, tags, attendance))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewLocalEvent {
    pub user_id: UserId,
    pub priority: Option<Priority>,
    pub tags: Vec<String>,
    // Event data
    pub starts_at: i64,
    pub all_day: bool,
    pub duration: Option<i32>,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}

use crate::models::ics_source::deserialize_checkbox;
use crate::models::ics_source::serialize_checkbox;

use super::EventId;
use super::EventLike;
use super::EventSource;
use super::Priority;
use super::SourceLocal;
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct LocalEventForm {
    pub summary: String,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub priority: Option<Priority>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub tags: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub description: Option<String>,
    pub starts_at: String,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub starts_at_tz: Option<i8>,
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub all_day: bool,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub duration_h: Option<i32>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub duration_m: Option<i32>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub duration_s: Option<i32>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub location: Option<String>,

    #[serde(flatten)]
    pub attendance: AttendanceForm,
}

pub type FormWithUser<'a> = (LocalEventForm, &'a UserPublic);
impl<'a> From<FormWithUser<'a>> for NewLocalEvent {
    fn from((form, user): FormWithUser) -> Self {
        let raw_tz = form.starts_at_tz.unwrap_or(user.interface_timezone_h);
        let mut tags = vec![];
        if let Some(tags_str) = form.tags.as_ref() {
            tags = tags_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        let starts_at = crate::utils::time::from_form(&form.starts_at, raw_tz).timestamp();

        let duration = match (form.duration_h, form.duration_m, form.duration_s) {
            (None, None, None) => None,
            _ => Some(
                form.duration_s.unwrap_or_default()
                    + form.duration_m.unwrap_or_default() * 60
                    + form.duration_h.unwrap_or_default() * 3600,
            ),
        };

        // generate a unique identifier
        let uid = format!("{}:{}@olmonoko", uuid::Uuid::new_v4(), user.id);

        Self {
            user_id: user.id,
            starts_at,
            priority: form.priority,
            tags,
            all_day: form.all_day,
            duration,
            summary: form.summary,
            description: form.description,
            location: form.location,
            uid,
        }
    }
}
impl From<LocalEvent> for LocalEventForm {
    fn from(event: LocalEvent) -> Self {
        let duration_h = event.duration.map(|d| d / 3600);
        let duration_m = event.duration.map(|d| (d % 3600) / 60);
        let duration_s = event.duration.map(|d| (d % 3600) % 60);
        Self {
            priority: event.priority,
            tags: if event.tags.is_empty() {
                None
            } else {
                Some(event.tags.join(", "))
            },
            summary: event.summary,
            description: event.description,
            starts_at: crate::utils::time::to_form(event.starts_at).unwrap_or_default(),
            starts_at_tz: Some(0),
            all_day: event.all_day,
            duration_h,
            duration_m,
            duration_s,
            location: event.location,
            attendance: AttendanceForm::default(),
        }
    }
}
type LocalEventWithAttendance = (LocalEvent, Option<Attendance>);
impl From<LocalEventWithAttendance> for LocalEventForm {
    fn from((event, attendance): LocalEventWithAttendance) -> Self {
        let mut form = LocalEventForm::from(event);
        if let Some(attendance) = attendance {
            form.attendance = AttendanceForm::from(attendance);
        }
        form
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::models::user::{RawUser, User};

    fn test_user() -> UserPublic {
        UserPublic::from(User::from(RawUser {
            id: 1,
            interface_timezone: "UTC".to_string(),
            email: "tester@olmonoko.ruta.fi".to_string(),
            admin: false,
            created_at: 0,
            password_hash: "abc".to_string(),
        }))
    }

    #[test]
    fn parse_form_utc() {
        let form = LocalEventForm {
            priority: None,
            tags: Some("1, 2, 3a , 4 5".to_string()),
            summary: "Test".to_string(),
            description: Some("Test".to_string()),
            starts_at: "2021-01-01T00:00:00".to_string(),
            starts_at_tz: Some(0),
            all_day: false,
            duration_h: Some(1),
            duration_m: Some(30),
            duration_s: Some(5),
            location: Some("Test".to_string()),
            attendance: AttendanceForm::default(),
        };
        let event = NewLocalEvent::from((form, &test_user()));
        assert_eq!(event.user_id, 1);
        assert_eq!(event.priority, None);
        assert_eq!(event.tags, vec!["1", "2", "3a", "4 5"]);
        assert_eq!(event.summary, "Test");
        assert_eq!(event.description, Some("Test".to_string()));
        assert_eq!(event.starts_at, 1609459200);
        assert_eq!(event.duration, Some(3600 + 30 * 60 + 5));
        assert_eq!(event.location, Some("Test".to_string()));
    }

    #[test]
    fn parse_form_helsinki() {
        let form = LocalEventForm {
            priority: None,
            tags: None,
            summary: "Test".to_string(),
            description: Some("Test".to_string()),
            starts_at: "2021-01-01T00:00:00".to_string(),
            starts_at_tz: Some(2),
            all_day: false,
            duration_h: Some(1),
            duration_s: None,
            duration_m: None,
            location: Some("Test".to_string()),
            attendance: AttendanceForm::default(),
        };
        let event = NewLocalEvent::from((form, &test_user()));
        assert_eq!(event.user_id, 1);
        assert_eq!(event.summary, "Test");
        assert_eq!(event.description, Some("Test".to_string()));
        assert_eq!(event.starts_at, 1609452000);
        assert_eq!(event.duration, Some(3600));
        assert_eq!(event.location, Some("Test".to_string()));
    }

    #[test]
    fn parse_stupid_browser() {
        let form = LocalEventForm {
            priority: None,
            tags: None,
            summary: "Test".to_string(),
            description: Some("Test".to_string()),
            starts_at: "2021-01-01T00:00".to_string(),
            starts_at_tz: Some(-2),
            all_day: false,
            duration_s: Some(3600),
            duration_m: None,
            duration_h: None,
            location: Some("Test".to_string()),
            attendance: AttendanceForm::default(),
        };
        let event = NewLocalEvent::from((form, &test_user()));
        assert_eq!(event.user_id, 1);
        assert_eq!(event.summary, "Test");
        assert_eq!(event.description, Some("Test".to_string()));
        assert_eq!(event.starts_at, 1609466400);
        assert_eq!(event.duration, Some(3600));
        assert_eq!(event.location, Some("Test".to_string()));
    }
}

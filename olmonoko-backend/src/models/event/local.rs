use std::vec;

use chrono::Utc;

use serde_with::As;
use serde_with::NoneAsEmptyString;

use crate::models::bills::AutoDescription;
use crate::models::bills::Bill;
use crate::models::bills::RawBill;
use crate::utils::time::from_timestamp;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawLocalEvent {
    pub id: i64,
    pub user_id: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub priority: Option<i64>,
    // Event data
    pub starts_at: i64,
    pub all_day: bool,
    pub duration: Option<i64>,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
impl EventLike for RawLocalEvent {
    fn id(&self) -> i64 {
        self.id
    }
    fn source(&self) -> EventSource {
        EventSource::Local(SourceLocal {
            user_id: self.user_id,
        })
    }
    fn priority(&self) -> Option<i64> {
        self.priority
    }
    fn tags(&self) -> Vec<String> {
        vec![] // FIX: This is ridiculous, the trait is getting unusable for raw events
    }
    fn all_day(&self) -> bool {
        self.all_day
    }
    fn duration(&self) -> Option<i64> {
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
    pub id: i64,
    pub user_id: i64,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub priority: Option<i64>,
    pub tags: Vec<String>,
    // Event data
    pub starts_at: chrono::DateTime<Utc>,
    pub all_day: bool,
    pub duration: Option<i64>,
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
impl From<(RawLocalEvent, Vec<String>)> for LocalEvent {
    fn from((raw, tags): (RawLocalEvent, Vec<String>)) -> Self {
        Self {
            id: raw.id,
            user_id: raw.user_id,
            created_at: from_timestamp(raw.created_at),
            updated_at: from_timestamp(raw.updated_at),
            priority: raw.priority,
            tags,
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
impl From<(RawLocalEvent, &str)> for LocalEvent {
    fn from((raw, tags_concat): (RawLocalEvent, &str)) -> Self {
        let tags: Vec<_> = tags_concat.split(',').map(|s| s.to_string()).collect();
        Self::from((raw, tags))
    }
}

impl From<(RawLocalEvent, Option<RawBill>, bool, Vec<String>)> for LocalEvent {
    fn from(
        (raw, bill, autodescription, tags): (RawLocalEvent, Option<RawBill>, bool, Vec<String>),
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
            starts_at: from_timestamp(raw.starts_at),
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
impl From<(RawLocalEvent, Option<RawBill>, bool, &str)> for LocalEvent {
    fn from(
        (raw, bill, autodescription, tags_concat): (RawLocalEvent, Option<RawBill>, bool, &str),
    ) -> Self {
        let tags: Vec<_> = tags_concat.split(',').map(|s| s.to_string()).collect();
        Self::from((raw, bill, autodescription, tags))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewLocalEvent {
    pub user_id: i64,
    pub priority: Option<i64>,
    pub tags: Vec<String>,
    // Event data
    pub starts_at: i64,
    pub all_day: bool,
    pub duration: Option<i64>,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}

use crate::models::ics_source::deserialize_checkbox;
use crate::models::ics_source::serialize_checkbox;

use super::EventLike;
use super::EventSource;
use super::SourceLocal;
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct LocalEventForm {
    pub summary: String,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub priority: Option<i64>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub tags: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub description: Option<String>,
    pub starts_at: String,
    pub starts_at_tz: i8,
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub all_day: bool,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub duration: Option<i64>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub location: Option<String>,
}

pub type FormWithUserId = (LocalEventForm, i64);
impl From<FormWithUserId> for NewLocalEvent {
    fn from((form, user_id): FormWithUserId) -> Self {
        let mut tags = vec![];
        if let Some(tags_str) = form.tags.as_ref() {
            tags = tags_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        // FIX: This is stupid
        let dt = if form.starts_at.chars().filter(|c| *c == ':').count() == 2 {
            form.starts_at.clone()
        } else {
            format!("{}:00", form.starts_at)
        };
        let tz = if form.starts_at_tz >= 0 {
            format!("+{:02}:00", form.starts_at_tz)
        } else {
            format!("-{:02}:00", -form.starts_at_tz)
        };
        let rfc = format!("{dt}{tz}");
        tracing::debug!("Parsing RFC3339 datetime: {}", rfc);
        let starts_at =
            chrono::DateTime::parse_from_rfc3339(&rfc).expect("Failed to parse RFC3339 datetime");
        let starts_at = starts_at.with_timezone(&Utc).timestamp();

        // generate a unique identifier
        let uid = format!("{}:{}@olmonoko", uuid::Uuid::new_v4(), user_id);

        Self {
            user_id,
            starts_at,
            priority: form.priority,
            tags,
            all_day: form.all_day,
            duration: form.duration,
            summary: form.summary,
            description: form.description,
            location: form.location,
            uid,
        }
    }
}
impl From<LocalEvent> for LocalEventForm {
    fn from(event: LocalEvent) -> Self {
        Self {
            priority: event.priority,
            tags: if event.tags.is_empty() {
                None
            } else {
                Some(event.tags.join(", "))
            },
            summary: event.summary,
            description: event.description,
            starts_at: event
                .starts_at
                .to_rfc3339()
                .split('+')
                .collect::<Vec<_>>()
                .first()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            starts_at_tz: 0,
            all_day: event.all_day,
            duration: event.duration,
            location: event.location,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn parse_form_utc() {
        let form = LocalEventForm {
            priority: None,
            tags: Some("1, 2, 3a , 4 5".to_string()),
            summary: "Test".to_string(),
            description: Some("Test".to_string()),
            starts_at: "2021-01-01T00:00:00".to_string(),
            starts_at_tz: 0,
            all_day: false,
            duration: Some(3600),
            location: Some("Test".to_string()),
        };
        let (form, user_id) = (form, 1);
        let event = NewLocalEvent::from((form, user_id));
        assert_eq!(event.user_id, 1);
        assert_eq!(event.priority, None);
        assert_eq!(event.tags, vec!["1", "2", "3a", "4 5"]);
        assert_eq!(event.summary, "Test");
        assert_eq!(event.description, Some("Test".to_string()));
        assert_eq!(event.starts_at, 1609459200);
        assert_eq!(event.duration, Some(3600));
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
            starts_at_tz: 2,
            all_day: false,
            duration: Some(3600),
            location: Some("Test".to_string()),
        };
        let (form, user_id) = (form, 1);
        let event = NewLocalEvent::from((form, user_id));
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
            starts_at_tz: -2,
            all_day: false,
            duration: Some(3600),
            location: Some("Test".to_string()),
        };
        let (form, user_id) = (form, 1);
        let event = NewLocalEvent::from((form, user_id));
        assert_eq!(event.user_id, 1);
        assert_eq!(event.summary, "Test");
        assert_eq!(event.description, Some("Test".to_string()));
        assert_eq!(event.starts_at, 1609466400);
        assert_eq!(event.duration, Some(3600));
        assert_eq!(event.location, Some("Test".to_string()));
    }
}

use chrono::Utc;
use std::hash::Hash;
use std::hash::Hasher;

use crate::{models::attendance::Attendance, utils::time::from_timestamp};

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawRemoteEvent {
    pub id: i64,
    pub event_source_id: i64,
    pub event_source_priority: i64,
    pub priority_override: Option<i64>,
    // Event data
    pub rrule: Option<String>,
    pub dt_stamp: Option<i64>,
    pub all_day: bool,
    pub duration: Option<i64>,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteEvent {
    pub id: i64,
    pub event_source_id: i64,
    pub priority: Option<i64>,
    pub attendance: Option<Attendance>,
    // Event data
    pub rrule: Option<String>,
    pub dt_stamp: Option<chrono::DateTime<Utc>>,
    pub duration: Option<i64>,
    pub all_day: bool,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
impl From<(RawRemoteEvent, Option<Attendance>)> for RemoteEvent {
    fn from((raw, attendance): (RawRemoteEvent, Option<Attendance>)) -> Self {
        let priority = if let Some(priority_override) = raw.priority_override {
            priority_override
        } else {
            raw.event_source_priority
        };
        let priority = if priority == 0 { None } else { Some(priority) };
        Self {
            id: raw.id,
            event_source_id: raw.event_source_id,
            attendance,
            priority,
            rrule: raw.rrule,
            dt_stamp: raw.dt_stamp.map(from_timestamp),
            all_day: raw.all_day,
            duration: raw.duration,
            summary: raw.summary,
            description: raw.description,
            location: raw.location,
            uid: raw.uid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NewRemoteEvent {
    pub event_source_id: i64,
    pub priority_override: Option<i64>,
    // Event data
    pub rrule: Option<String>,
    pub dt_stamp: Option<i64>,
    pub all_day: bool,
    pub duration: Option<i64>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
    // tags
    pub tags: Vec<String>,
}
impl Hash for NewRemoteEvent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // everything except dt_stamp, as that often changes for every sync of the source
        self.event_source_id.hash(state);
        self.priority_override.hash(state);
        self.rrule.hash(state);
        self.all_day.hash(state);
        self.duration.hash(state);
        self.summary.hash(state);
        self.description.hash(state);
        self.location.hash(state);
        self.uid.hash(state);
        self.tags.hash(state);
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RemoteEventOccurrence {
    pub id: i64,
    pub event_id: i64,
    pub from_rrule: bool,
    pub starts_at: chrono::DateTime<Utc>,
}
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize, PartialEq, Hash)]
pub struct NewRemoteEventOccurrence {
    pub event_id: i64,
    pub starts_at: i64,
    pub from_rrule: bool,
}

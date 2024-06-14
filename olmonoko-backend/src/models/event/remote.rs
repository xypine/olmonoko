use chrono::Utc;

use crate::utils::time::from_timestamp;

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
impl From<RawRemoteEvent> for RemoteEvent {
    fn from(raw: RawRemoteEvent) -> Self {
        let priority = if let Some(priority_override) = raw.priority_override {
            priority_override
        } else {
            raw.event_source_priority
        };
        let priority = if priority == 0 { None } else { Some(priority) };
        Self {
            id: raw.id,
            event_source_id: raw.event_source_id,
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
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RemoteEventOccurrence {
    pub id: i64,
    pub event_id: i64,
    pub from_rrule: bool,
    pub starts_at: chrono::DateTime<Utc>,
}
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct NewRemoteEventOccurrence {
    pub event_id: i64,
    pub starts_at: i64,
    pub from_rrule: bool,
}

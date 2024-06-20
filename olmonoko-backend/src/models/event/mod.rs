pub mod local;
pub mod remote;

use chrono::{TimeZone, Timelike, Utc};
use chrono_humanize::Tense;

use crate::utils::time::from_timestamp;

use self::{local::LocalEvent, remote::RemoteEvent};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SourceLocal {
    pub user_id: i64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SourceRemote {
    pub source_id: i64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum EventSource {
    Local(SourceLocal),
    Remote(SourceRemote),
}

#[allow(dead_code)]
pub trait EventLike {
    fn id(&self) -> i64;
    fn source(&self) -> EventSource;
    fn all_day(&self) -> bool;
    fn duration(&self) -> Option<i64>;
    fn summary(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn location(&self) -> Option<&str>;
    fn priority(&self) -> Option<i64>;
    fn tags(&self) -> Vec<String>;
}

pub const DEFAULT_PRIORITY: i64 = 5;
pub const PRIORITY_OPTIONS: [i64; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub id: i64,
    pub source: EventSource,
    pub priority: i64,
    pub tags: Vec<String>,
    // Event data
    pub starts_at: Vec<chrono::DateTime<Utc>>,
    pub all_day: bool,
    pub duration: Option<i64>,
    pub rrule: Option<String>,

    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
impl EventLike for Event {
    fn id(&self) -> i64 {
        self.id
    }
    fn source(&self) -> EventSource {
        self.source
    }
    fn priority(&self) -> Option<i64> {
        Some(self.priority)
    }
    fn tags(&self) -> Vec<String> {
        self.tags.clone()
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

impl From<LocalEvent> for Event {
    fn from(local: LocalEvent) -> Self {
        Self {
            id: local.id,
            source: EventSource::Local(SourceLocal {
                user_id: local.user_id,
            }),
            priority: local.priority.unwrap_or(DEFAULT_PRIORITY),
            tags: local.tags,
            starts_at: vec![local.starts_at],
            all_day: local.all_day,
            duration: local.duration,
            rrule: None,
            summary: local.summary,
            description: local.description,
            location: local.location,
            uid: local.uid,
        }
    }
}

impl From<(RemoteEvent, Vec<i64>)> for Event {
    fn from((remote, starts_at): (RemoteEvent, Vec<i64>)) -> Self {
        Self {
            id: remote.id,
            source: EventSource::Remote(SourceRemote {
                source_id: remote.event_source_id,
            }),
            priority: remote.priority.unwrap_or(DEFAULT_PRIORITY),
            tags: vec![], // TODO: Implement tags for remote events
            starts_at: starts_at.into_iter().map(from_timestamp).collect(),
            all_day: remote.all_day,
            duration: remote.duration,
            rrule: remote.rrule,
            summary: remote.summary,
            description: remote.description,
            location: remote.location,
            uid: remote.uid,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventOccurrence {
    pub id: i64, // event id, not specific to this occurrence
    pub source: EventSource,
    pub priority: i64,
    pub tags: Vec<String>,
    // Event data
    pub starts_at: chrono::DateTime<Utc>,
    pub all_day: bool,
    pub duration: Option<i64>,
    pub rrule: Option<String>,
    pub from_rrule: bool,

    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
impl EventLike for EventOccurrence {
    fn id(&self) -> i64 {
        self.id
    }
    fn source(&self) -> EventSource {
        self.source
    }
    fn priority(&self) -> Option<i64> {
        Some(self.priority)
    }
    fn tags(&self) -> Vec<String> {
        self.tags.clone()
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
impl From<Event> for Vec<EventOccurrence> {
    fn from(event: Event) -> Self {
        event
            .starts_at
            .into_iter()
            .enumerate()
            .map(|(i, starts_at)| EventOccurrence {
                id: event.id,
                source: event.source,
                priority: event.priority,
                tags: event.tags.clone(),
                starts_at,
                all_day: event.all_day,
                duration: event.duration,
                rrule: event.rrule.clone(),
                from_rrule: event.rrule.is_some() && i > 0,
                summary: event.summary.clone(),
                description: event.description.clone(),
                location: event.location.clone(),
                uid: event.uid.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EventOccurrenceHuman<T: TimeZone> {
    pub id: i64, // event id, not specific to this occurrence
    pub source: EventSource,
    pub priority: i64,
    pub tags: Vec<String>,
    // Event data
    #[serde(skip)]
    pub starts_at: chrono::DateTime<T>,
    pub starts_at_human: String,
    pub starts_at_seconds: i64,

    pub overlap_total: usize,
    pub overlap_index: usize,

    pub all_day: bool,
    pub duration: Option<i64>,
    pub duration_human: Option<String>,
    pub rrule: Option<String>,
    pub from_rrule: bool,

    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
impl<T: TimeZone> From<(EventOccurrence, &T)> for EventOccurrenceHuman<T>
where
    T::Offset: std::fmt::Display,
{
    fn from((occurrence, tz): (EventOccurrence, &T)) -> Self {
        let starts_at = occurrence.starts_at.with_timezone(tz);
        let starts_at_local = starts_at.naive_local();
        // starts_at_seconds is used for positioning the event on a given day
        let time = starts_at.time();
        let hours = time.hour() as i64;
        let minutes = time.minute() as i64;
        let seconds = time.second() as i64;
        let starts_at_seconds = if !occurrence.all_day {
            hours * 3600 + minutes * 60 + seconds
        } else {
            0
        };
        Self {
            id: occurrence.id,
            source: occurrence.source,
            priority: occurrence.priority,
            tags: occurrence.tags.clone(),

            starts_at_human: if occurrence.all_day {
                starts_at.format("%Y-%m-%d").to_string()
            } else {
                starts_at_local.to_string()
            },
            starts_at_seconds,
            starts_at,

            overlap_total: 1,
            overlap_index: 0,

            all_day: occurrence.all_day,
            duration: occurrence.duration,
            duration_human: occurrence.duration.map(|duration| {
                let ht = chrono_humanize::HumanTime::from(chrono::Duration::seconds(duration));
                ht.to_text_en(chrono_humanize::Accuracy::Precise, Tense::Present)
            }),
            rrule: occurrence.rrule.clone(),
            from_rrule: occurrence.from_rrule,
            summary: occurrence.summary,
            description: occurrence.description,
            location: occurrence.location,
            uid: occurrence.uid,
        }
    }
}
impl<T: TimeZone> EventLike for EventOccurrenceHuman<T> {
    fn id(&self) -> i64 {
        self.id
    }

    fn source(&self) -> EventSource {
        self.source
    }

    fn all_day(&self) -> bool {
        self.all_day
    }

    fn duration(&self) -> Option<i64> {
        self.duration
    }

    fn summary(&self) -> &str {
        self.summary.as_str()
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    fn priority(&self) -> Option<i64> {
        Some(self.priority)
    }

    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }
}

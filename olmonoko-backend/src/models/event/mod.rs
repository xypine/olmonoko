pub mod local;
pub mod remote;

use chrono::{TimeZone, Timelike, Utc};
use chrono_humanize::Tense;
use remote::{RemoteEventId, RemoteSourceId};

use crate::utils::time::from_timestamp;

use self::{local::LocalEvent, remote::RemoteEvent};

use super::{
    attendance::{Attendance, AttendanceForm},
    user::UserId,
};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SourceLocal {
    pub user_id: UserId,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SourceRemote {
    pub source_id: RemoteSourceId,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum EventSource {
    Local(SourceLocal),
    Remote(SourceRemote),
}

pub type EventId = i32;
#[allow(dead_code)]
pub trait EventLike {
    fn id(&self) -> EventId;
    fn source(&self) -> EventSource;
    fn all_day(&self) -> bool;
    fn starts_at(&self) -> Vec<i64>;
    fn duration(&self) -> Option<i32>;
    fn summary(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn location(&self) -> Option<&str>;
    fn priority(&self) -> Option<Priority>;
    fn tags(&self) -> Vec<String>;
}

pub type Priority = i32;
pub const DEFAULT_PRIORITY: Priority = 5;
pub const PRIORITY_OPTIONS: [Priority; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub id: EventId,
    pub source: EventSource,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub attendance: Option<Attendance>,
    // Event data
    pub starts_at: Vec<chrono::DateTime<Utc>>,
    pub all_day: bool,
    pub duration: Option<i32>,
    pub rrule: Option<String>,

    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
impl EventLike for Event {
    fn id(&self) -> EventId {
        self.id
    }
    fn source(&self) -> EventSource {
        self.source
    }
    fn priority(&self) -> Option<Priority> {
        Some(self.priority)
    }
    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }
    fn all_day(&self) -> bool {
        self.all_day
    }
    fn starts_at(&self) -> Vec<i64> {
        self.starts_at.iter().map(|s| s.timestamp()).collect()
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

impl From<LocalEvent> for Event {
    fn from(local: LocalEvent) -> Self {
        Self {
            id: local.id,
            source: EventSource::Local(SourceLocal {
                user_id: local.user_id,
            }),
            priority: local.priority.unwrap_or(DEFAULT_PRIORITY),
            tags: local.tags,
            attendance: local.attendance,
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
            attendance: remote.attendance,
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
    pub id: RemoteEventId, // event id, not specific to this occurrence
    pub source: EventSource,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub attendance: Option<Attendance>,
    // Event data
    pub starts_at: chrono::DateTime<Utc>,
    pub all_day: bool,
    pub duration: Option<i32>,
    pub rrule: Option<String>,
    pub from_rrule: bool,

    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
impl EventLike for EventOccurrence {
    fn id(&self) -> EventId {
        self.id
    }
    fn source(&self) -> EventSource {
        self.source
    }
    fn priority(&self) -> Option<Priority> {
        Some(self.priority)
    }
    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }
    fn all_day(&self) -> bool {
        self.all_day
    }
    fn starts_at(&self) -> Vec<i64> {
        vec![self.starts_at.timestamp()]
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
                attendance: event.attendance.clone(),
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventOccurrenceHuman {
    pub id: RemoteEventId, // event id, not specific to this occurrence
    pub source: EventSource,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub attendance: Option<Attendance>,
    pub attendance_form: Option<AttendanceForm>,
    // Event data
    pub starts_at_human: String,
    pub starts_at_seconds: i64,
    pub starts_at_utc: chrono::DateTime<Utc>,

    pub overlap_total: usize,
    pub overlap_index: usize,

    pub all_day: bool,
    pub duration: Option<i32>,
    pub duration_human: Option<String>,
    pub rrule: Option<String>,
    pub from_rrule: bool,

    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub uid: String,
}
impl EventOccurrenceHuman {
    // returns the start time in seconds since midnight and the duration in seconds
    // if the event doesn't span that day, returns None
    pub fn interface_span(&self, day_start: i64, day_end: i64) -> Option<(i64, Option<i32>)> {
        let start = self.starts_at_utc.timestamp();
        // check if the event starts after the day ends
        if self.starts_at_utc.timestamp() > day_end {
            return None;
        }

        if let Some(duration) = self.duration {
            let end = start + duration as i64;

            // check if the event ends before the day starts
            if end < day_start {
                return None;
            }

            let start = start.max(day_start);
            let end = end.min(day_end);

            let start = start - day_start;
            let end = end - day_start;

            let duration = end - start;
            if duration == 0 {
                return None;
            }

            Some((start, Some(duration as i32)))
        } else {
            if start < day_start {
                return None;
            }
            let start = start - day_start;
            Some((start, None))
        }
    }
}
impl<T: TimeZone> From<(EventOccurrence, &T)> for EventOccurrenceHuman
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
        let attendance_form = occurrence.attendance.clone().map(AttendanceForm::from);
        Self {
            id: occurrence.id,
            source: occurrence.source,
            priority: occurrence.priority,
            tags: occurrence.tags.clone(),
            attendance: occurrence.attendance.clone(),
            attendance_form,

            starts_at_human: if occurrence.all_day {
                starts_at.format("%Y-%m-%d").to_string()
            } else {
                starts_at_local.to_string()
            },
            starts_at_seconds,
            starts_at_utc: occurrence.starts_at,

            overlap_total: 1,
            overlap_index: 0,

            all_day: occurrence.all_day,
            duration: occurrence.duration,
            duration_human: occurrence.duration.map(|duration| {
                let ht =
                    chrono_humanize::HumanTime::from(chrono::Duration::seconds(duration as i64));
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
impl EventLike for EventOccurrenceHuman {
    fn id(&self) -> EventId {
        self.id
    }

    fn source(&self) -> EventSource {
        self.source
    }

    fn all_day(&self) -> bool {
        self.all_day
    }

    fn starts_at(&self) -> Vec<i64> {
        vec![self.starts_at_utc.timestamp()]
    }

    fn duration(&self) -> Option<i32> {
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

    fn priority(&self) -> Option<Priority> {
        Some(self.priority)
    }

    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn test_event_occurrence_span() {
        let event = EventOccurrence {
            id: 1,
            source: EventSource::Local(SourceLocal { user_id: 1 }),
            priority: 5,
            tags: vec![],
            attendance: None,
            starts_at: Utc.ymd(2021, 1, 1).and_hms(12, 0, 0),
            all_day: false,
            duration: Some(3600),
            rrule: None,
            from_rrule: false,
            summary: "Test".to_string(),
            description: None,
            location: None,
            uid: "test".to_string(),
        };
        let tz = Utc;
        let human = EventOccurrenceHuman::from((event, &tz));
        let day_start = Utc.ymd(2021, 1, 1).and_hms(0, 0, 0).timestamp();
        assert_eq!(
            human.interface_span(day_start, day_start + 86400),
            Some((12 * 3600, Some(3600)))
        );
        let day_start = Utc.ymd(2021, 1, 2).and_hms(0, 0, 0).timestamp();
        assert_eq!(human.interface_span(day_start, day_start + 86400), None);
    }

    #[test]
    fn test_event_occurrence_span_whole() {
        let event = EventOccurrence {
            id: 1,
            source: EventSource::Local(SourceLocal { user_id: 1 }),
            priority: 5,
            tags: vec![],
            attendance: None,
            starts_at: Utc.ymd(2021, 1, 1).and_hms(0, 0, 0),
            all_day: false,
            duration: Some(3600 * 24),
            rrule: None,
            from_rrule: false,
            summary: "Test".to_string(),
            description: None,
            location: None,
            uid: "test".to_string(),
        };
        let tz = Utc;
        let human = EventOccurrenceHuman::from((event, &tz));
        let day_start = Utc.ymd(2021, 1, 1).and_hms(0, 0, 0).timestamp();
        assert_eq!(
            human.interface_span(day_start, day_start + 86400),
            Some((0, Some(3600 * 24)))
        );
        let day_start = Utc.ymd(2021, 1, 2).and_hms(0, 0, 0).timestamp();
        assert_eq!(human.interface_span(day_start, day_start + 86400), None);
    }

    #[test]
    fn test_event_occurrence_span_multiday() {
        for tz in [chrono_tz::Etc::UTC, chrono_tz::Europe::Helsinki] {
            let event = EventOccurrence {
                id: 1,
                source: EventSource::Local(SourceLocal { user_id: 1 }),
                priority: 5,
                tags: vec![],
                attendance: None,
                starts_at: Utc.ymd(2021, 1, 1).and_hms(23, 30, 0),
                all_day: false,
                duration: Some(3600),
                rrule: None,
                from_rrule: false,
                summary: "Test".to_string(),
                description: None,
                location: None,
                uid: "test".to_string(),
            };
            let human = EventOccurrenceHuman::from((event, &tz));
            let day_start = Utc.ymd(2021, 1, 1).and_hms(0, 0, 0).timestamp();
            assert_eq!(
                human.interface_span(day_start, day_start + 86400),
                Some(((23.5 * 3600.0) as i64, Some(3600 / 2)))
            );
            let day_start = Utc.ymd(2021, 1, 2).and_hms(0, 0, 0).timestamp();
            assert_eq!(
                human.interface_span(day_start, day_start + 86400),
                Some((0, Some(3600 / 2)))
            );

            let event = EventOccurrence {
                id: 1,
                source: EventSource::Local(SourceLocal { user_id: 1 }),
                priority: 5,
                tags: vec![],
                attendance: None,
                starts_at: Utc.ymd(2024, 7, 25).and_hms(0, 0, 0),
                all_day: true,
                duration: Some(3600 * 24 * 3),
                rrule: None,
                from_rrule: false,
                summary: "Test".to_string(),
                description: None,
                location: None,
                uid: "test".to_string(),
            };
            let human = EventOccurrenceHuman::from((event, &tz));
            let day_start = Utc.ymd(2024, 7, 26).and_hms(0, 0, 0).timestamp();
            assert_eq!(
                human.interface_span(day_start, day_start + 86400),
                Some((0, Some(3600 * 24)))
            );
            let day_start = Utc.ymd(2024, 7, 28).and_hms(0, 0, 0).timestamp();
            assert_eq!(human.interface_span(day_start, day_start + 86400), None);
        }
    }

    #[test]
    fn test_event_occurrence_span_no_duration() {
        let event = EventOccurrence {
            id: 1,
            source: EventSource::Local(SourceLocal { user_id: 1 }),
            priority: 5,
            tags: vec![],
            starts_at: Utc.ymd(2021, 1, 1).and_hms(12, 0, 0),
            all_day: false,
            duration: None,
            rrule: None,
            from_rrule: false,
            summary: "Test".to_string(),
            description: None,
            location: None,
            uid: "test".to_string(),
            attendance: None,
        };
        let tz = Utc;
        let human = EventOccurrenceHuman::from((event, &tz));
        let day_start = Utc.ymd(2021, 1, 1).and_hms(0, 0, 0).timestamp();
        assert_eq!(
            human.interface_span(day_start, day_start + 86400),
            Some((12 * 3600, None))
        );
        let day_start = Utc.ymd(2021, 1, 2).and_hms(0, 0, 0).timestamp();
        assert_eq!(human.interface_span(day_start, day_start + 86400), None);
    }
}

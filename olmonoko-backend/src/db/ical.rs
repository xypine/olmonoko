//! Enhancements for the icalendar-rs library

use chrono::{DateTime, Duration, NaiveTime, Utc};
use icalendar::{Component, DatePerhapsTime};

pub trait EnhancedIcalendarEvent {
    fn get_end_auto(&self) -> Option<DateTime<Utc>>;
    fn get_duration(&self) -> Option<Duration>;
}

pub(crate) fn parse_duration(s: &str) -> Option<Duration> {
    iso8601::duration(s)
        .ok()
        .and_then(|iso| Duration::from_std(iso.into()).ok())
}

impl EnhancedIcalendarEvent for icalendar::Event {
    fn get_duration(&self) -> Option<Duration> {
        parse_duration(self.properties().get("DURATION")?.value())
    }

    fn get_end_auto(&self) -> Option<DateTime<Utc>> {
        fn flatten_caldt(dt: DatePerhapsTime) -> Option<DateTime<Utc>> {
            match dt {
                DatePerhapsTime::Date(date) => date.and_time(NaiveTime::MIN).into(),
                DatePerhapsTime::DateTime(dt) => dt,
            }
            .try_into_utc()
        }
        if let Some(end) = self.get_end() {
            return flatten_caldt(end);
        }
        let start = flatten_caldt(self.get_start()?)?;
        let duration = self.get_duration()?;
        let end = start + duration;
        Some(end)
    }
}

//! Enhancements for the icalendar-rs library

use chrono::{DateTime, Duration, NaiveTime, Utc};
use chrono_tz::Tz;
use icalendar::{Component, DatePerhapsTime};

pub trait EnhancedIcalendarEvent {
    fn is_all_day(&self) -> bool;
    fn get_end_auto(&self, src_tz: Tz) -> Option<DateTime<Utc>>;
    fn get_duration(&self) -> Option<Duration>;
}

pub(crate) fn parse_duration(s: &str) -> Option<Duration> {
    iso8601::duration(s)
        .ok()
        .and_then(|iso| Duration::from_std(iso.into()).ok())
}

impl EnhancedIcalendarEvent for icalendar::Event {
    fn is_all_day(&self) -> bool {
        self.get_start()
            .map(|dt| matches!(dt, DatePerhapsTime::Date(_)))
            .unwrap_or(false) // events without occurrences aren't "all day" either
    }

    fn get_duration(&self) -> Option<Duration> {
        parse_duration(self.properties().get("DURATION")?.value())
    }

    fn get_end_auto(&self, src_tz: Tz) -> Option<DateTime<Utc>> {
        fn flatten_caldt(dt: DatePerhapsTime, src_tz: Tz) -> Option<DateTime<Utc>> {
            match dt {
                DatePerhapsTime::Date(date) => date
                    .and_time(NaiveTime::MIN)
                    .and_local_timezone(src_tz)
                    .earliest()
                    .map(|dt| dt.to_utc()),
                DatePerhapsTime::DateTime(dt) => dt.try_into_utc(),
            }
        }
        if let Some(end) = self.get_end() {
            return flatten_caldt(end, src_tz);
        }
        let start = flatten_caldt(self.get_start()?, src_tz)?;
        let duration = self.get_duration()?;
        let end = start + duration;
        Some(end)
    }
}

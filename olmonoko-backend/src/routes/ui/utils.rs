use std::collections::HashMap;

use actix_web::web::Data;
use chrono::{DateTime, Datelike, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use itertools::Itertools;
use olmonoko_common::{models::{event::{EventOccurrence, EventOccurrenceHuman}, user::UserId}, utils::{event_filters::EventFilter, ui::arrange}, AppState};

use crate::db_utils::events::get_visible_event_occurrences;

use super::INTERFACE_MIN_EVENT_LENGTH;

pub struct CalendarWidgetData {
    pub events: Vec<EventOccurrenceHuman>,
    pub current_day: Option<usize>,
    pub current_time_seconds: u32,
    pub events_by_day: [Vec<EventOccurrenceHuman>; 7],
    pub prev_year: i32,
    pub prev_week: u32,
    pub next_year: i32,
    pub next_week: u32,
    pub week_dates: Vec<String>,
    pub day_names: [&'static str; 7],
    pub selected_year: i32,
    pub selected_week: u32,
}
impl CalendarWidgetData {
    pub fn insert_into_context(&self, context: &mut tera::Context) {
        context.insert("events", &self.events);
        context.insert("current_day", &self.current_day);
        context.insert("current_time_seconds", &self.current_time_seconds);
        context.insert("events_by_day", &self.events_by_day);

        context.insert("prev_year", &self.prev_year);
        context.insert("prev_week", &self.prev_week);
        context.insert("next_year", &self.next_year);
        context.insert("next_week", &self.next_week);
        // generate dates for the current week
        for (i, d) in self.week_dates.iter().enumerate() {
            context.insert(format!("week_date_{}", i), d);
        }
        context.insert("week_dates", &self.week_dates);
        context.insert(
            "day_names",
            &self.day_names
        );

        context.insert("selected_year", &self.selected_year);
        context.insert("selected_week", &self.selected_week);
    }
}
pub async fn build_calendar(data: &Data<AppState>, user_id: UserId, pivot: DateTime<Utc>, display_tz: Tz, filter: EventFilter) -> CalendarWidgetData {
    let now = chrono::Utc::now().with_timezone(&display_tz);

    let mut filter = filter;

    let pivot_local = pivot
        .with_timezone(&display_tz)
        .with_time(NaiveTime::MIN)
        .earliest()
        .expect("Failed to convert pivot to local time");


    // after yesterday (from today)
    let from = (pivot - chrono::Duration::milliseconds(1)).timestamp();
    // before next week
    let to = (pivot + chrono::Duration::days(7)).timestamp();
    filter.after = Some(from);
    filter.before = Some(to);
    let events = get_visible_event_occurrences(&data, Some(user_id), true, &filter).await;
    let linked_local_events: Vec<_> = events
        .iter()
        .flat_map(|e| e.linked_events.clone())
        .collect();
    let mut linked_local_events_map = HashMap::new();
    for linked in linked_local_events {
        let res = events.iter().find(|e| e.id == linked);
        if let Some(event) = res {
            linked_local_events_map.insert(linked, event.clone());
        }
    }
    // humanize dates etc
    let events = events
        .into_iter()
        .map(|e| EventOccurrenceHuman::from((e, &display_tz)))
        .filter(|e| match e.id {
            olmonoko_common::models::event::EventId::Local(_) => true,
            olmonoko_common::models::event::EventId::Remote(_) => {
                e.linked_events.is_empty()
                    || e.linked_events.iter().any(|linked| {
                        if let Some(result) = linked_local_events_map.get(linked) {
                            if result.starts_at != e.starts_at_utc {
                                tracing::warn!("starts_at do not match {}, {}", linked, e.id);
                                return true;
                            }
                            if result.duration != e.duration {
                                tracing::warn!("duration do not match {}, {}", linked, e.id);
                                return true;
                            }
                            return false;
                        }
                        tracing::warn!("Linked local event {} not found for {}", linked, e.id);
                        true
                    })
            }
        })
        .collect::<Vec<_>>();

    let current_day: Option<usize> = if now.iso_week() == pivot.iso_week() {
        Some(now.weekday().number_from_monday() as usize - 1)
    } else {
        None
    };
    let current_time = now.time();
    let current_time_seconds = current_time.hour() * 3600 + current_time.minute() * 60;
    let mut events_by_day: [Vec<_>; 7] = Default::default();
    for day in 0..7 {
        let mut day_events = events
            .iter()
            .filter_map(|event| {
                let mut event = event.clone();
                let today_ts = pivot_local.timestamp() + (day * 24 * 3600) as i64;
                let tomorrow_ts = today_ts + (24 * 3600) - 1;
                if let Some((starts_at_s, duration)) =
                    event.interface_span(today_ts, tomorrow_ts)
                {
                    event.starts_at_seconds = starts_at_s;
                    event.duration = duration;
                    return Some(event);
                }
                None
            })
            .sorted_by_key(|event| event.priority)
            .collect::<Vec<_>>();

        let is_today = current_day == Some(day as usize);
        if is_today {
            day_events.push(EventOccurrenceHuman {
                id: olmonoko_common::models::event::EventId::Local(-1),
                source: olmonoko_common::models::event::EventSource::Local(
                    olmonoko_common::models::event::SourceLocal { user_id: -1 },
                ),
                linked_events: vec![],
                priority: 1,
                starts_at_utc: now.with_timezone(&chrono::Utc),
                starts_at_human: "".to_string(),
                starts_at_seconds: current_time_seconds as i64,
                overlap_total: 0,
                overlap_index: 0,
                all_day: false,
                duration: None,
                duration_human: None,
                rrule: None,
                from_rrule: false,
                summary: "".to_string(),
                description: None,
                location: None,
                uid: "olmonoko::now".to_string(),
                occurrence_id: None,
            })
        }

        for event in &mut day_events {
            // normalize all day events to start at 00:00 and last the default amount
            if event.all_day {
                event.starts_at_seconds = 0;
                event.duration = None;
            }
            // set event duration to be at least 1 hour
            if event.duration.unwrap_or(0) < INTERFACE_MIN_EVENT_LENGTH {
                event.duration = Some(INTERFACE_MIN_EVENT_LENGTH);
            }
        }

        // find overlapping events and adjust event overlap_count and overlap_index
        let starts_at: Vec<_> = day_events.iter().map(|e| e.starts_at_seconds).collect();
        let durations: Vec<_> = day_events
            .iter()
            .map(|e| e.duration.unwrap_or_default())
            .collect();
        let arrangements = arrange(starts_at.as_slice(), durations.as_slice());
        for (i, a) in arrangements.iter().enumerate() {
            day_events[i].overlap_index = a.lane as usize;
            day_events[i].overlap_total = a.width as usize;
        }

        events_by_day[day as usize] = day_events;
    }

    // generate data for the year and month selectors
    let before = pivot - chrono::Duration::weeks(1);
    let after = pivot + chrono::Duration::weeks(1);
    let week_before = before.iso_week();
    let week_after = after.iso_week();
    let year_before = week_before.year();
    let year_after = week_after.year();

    // generate dates for the current week
    let week_dates = (0..7)
        .map(|i| pivot + chrono::Duration::days(i))
        .map(|day| {
            day.format("%d.%-m.").to_string()
        })
        .collect();


    CalendarWidgetData {
        events,
        events_by_day,
        selected_week: pivot.iso_week().week(),
        selected_year: pivot.iso_week().year(),
        prev_year: year_before,
        prev_week: week_before.week(),
        next_year: year_after,
        next_week: week_after.week(),
        day_names: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
        week_dates,
        current_day,
        current_time_seconds
    }
}

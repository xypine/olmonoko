use icalendar::{Component, EventLike};

use olmonoko_common::{
    models::event::EventOccurrence,
    utils::time::{from_timestamp, get_current_time},
};

pub mod scheduler;
pub mod source_processing;

pub(crate) async fn compose_ics(
    events: Vec<EventOccurrence>,
) -> Result<String, Box<dyn std::error::Error>> {
    let dt_stamp = get_current_time();
    let mut calendar = icalendar::Calendar::new();
    calendar.timezone("UTC"); // All timestamps have been converted to UTC
    for event in events.iter() {
        // We don't want to pollute the ics with occurrences covered by rrule
        if event.from_rrule {
            continue;
        }

        let mut ical_event = icalendar::Event::new();
        // ical_event.uid(&format!("{}@olmonoko", event.uid));
        ical_event.uid(&event.uid);
        ical_event.timestamp(dt_stamp);
        ical_event.summary(&event.summary);
        if event.all_day {
            ical_event.starts(event.starts_at.date_naive());
        } else {
            ical_event.starts(event.starts_at);
        }
        if let Some(duration) = event.duration {
            let end = from_timestamp(event.starts_at.timestamp() + duration as i64);
            if event.all_day {
                ical_event.ends(end.date_naive());
            } else {
                ical_event.ends(end);
            }
        }
        if let Some(description) = &event.description {
            ical_event.description(description);
        }
        if let Some(rrule) = &event.rrule {
            ical_event.add_property("RRULE", rrule);
        }
        if event.priority > 0 && event.priority < 10 {
            ical_event.priority(event.priority as u32);
        } else {
            let event_id = event.id.to_string();
            tracing::warn!(event_id, "Invalid event priority: {}", event.priority);
        }
        // FIX: Populate these from occurrences
        // if let Some(dt_start) = event.dt_start {
        //     let ts = dt_start.parse::<i64>().expect("Failed to parse timestamp");
        //     let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap();
        //     ical_event.starts(dt);
        // }
        // if let Some(dt_end) = event.dt_end {
        //     let ts = dt_end.parse::<i64>().expect("Failed to parse timestamp");
        //     let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap();
        //     ical_event.ends(dt);
        // }
        if let Some(location) = &event.location {
            ical_event.location(location);
        }
        // if let Some(dt_stamp) = event.dt_stamp {
        //     let ts = dt_stamp.parse::<i64>().expect("Failed to parse timestamp");
        //     ical_event.timestamp(chrono::NaiveDateTime::from_timestamp(ts, 0));
        // }
        calendar.push(ical_event.done());
    }

    let ics = calendar.to_string();

    Ok(ics)
}

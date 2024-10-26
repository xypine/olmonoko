use std::collections::HashMap;

use actix_web::{
    cookie::SameSite,
    get,
    web::{self, Query},
    HttpRequest, HttpResponse, HttpResponseBuilder, Responder, Scope,
};
use chrono::{Datelike, NaiveTime, Timelike};
use itertools::Itertools;
use olmonoko_common::{
    models::{
        api_key::{ApiKey, ApiKeyForm, RawApiKey},
        event::{
            local::{LocalEvent, LocalEventForm, LocalEventId},
            remote::RemoteEventOccurrenceId,
            EventOccurrenceHuman, Priority,
        },
        user::{RawUser, TimezoneEntity, UnverifiedUser, UserPublic},
    },
    utils::{
        event_filters::{EventFilter, RawEventFilter, RawEventFilterWithDate},
        flash::FLASH_COOKIE_NAME,
        time::{from_timestamp, get_current_time},
        ui::arrange,
    },
    AppState,
};

fn remove_flash_cookie(mut builder: HttpResponseBuilder) -> HttpResponseBuilder {
    let mut removal_cookie = actix_web::cookie::Cookie::build(FLASH_COOKIE_NAME, "")
        .path("/")
        .same_site(SameSite::Strict)
        .finish();
    removal_cookie.make_removal();
    builder.cookie(removal_cookie);
    builder
}

fn admin_check(user: Option<UserPublic>) -> Option<HttpResponse> {
    if let Some(user) = user {
        if !user.admin {
            return Some(HttpResponse::Forbidden().body("This page requires admin access"));
        }
        None // User is an admin
    } else {
        Some(HttpResponse::Unauthorized().body("This page requires you to be signed in"))
    }
}

#[get("/remote")]
async fn sources(data: web::Data<AppState>, request: HttpRequest) -> impl Responder {
    let (mut context, user, _key) = request.get_session_context(&data).await;
    let all_sources = get_visible_sources_with_event_count(&data, user.map(|u| u.id)).await;
    context.insert("sources", &all_sources);

    let content = data
        .templates
        .render("pages/sources.html", &context)
        .unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}

#[derive(Debug, serde::Deserialize)]
struct LocalQuery {
    selected: Option<LocalEventId>,
    from_occurrence: Option<RemoteEventOccurrenceId>,
    #[serde(flatten)]
    filter: RawEventFilterWithDate,
}
#[get("/local")]
async fn local(
    data: web::Data<AppState>,
    request: HttpRequest,
    query: Query<LocalQuery>,
) -> Result<impl Responder, AnyInternalServerError> {
    let (mut context, user, _key) = request.get_session_context(&data).await;
    if let Some(user) = user {
        context.insert("filter", &query.filter);
        context.insert("filter_set", &query.filter.is_defined());
        let filter = EventFilter::from(query.filter.clone());
        let events = get_user_local_events(&data, user.id, false, &filter).await;
        let available_tags = events
            .iter()
            .flat_map(|event| event.tags.iter())
            .unique()
            .sorted()
            .collect::<Vec<_>>();
        let events_grouped_by_priority = events
            .clone()
            .into_iter()
            .sorted_by_key(|event| event.priority)
            .chunk_by(|event| event.priority)
            .into_iter()
            .map(|(priority, group)| {
                let group: Vec<_> = group.sorted_by_key(|e| e.starts_at.timestamp()).collect();
                (priority, group)
            })
            .collect::<Vec<_>>();
        let selected = query.selected.and_then(|selected_event_id| {
            events
                .iter()
                .find(|event| event.id == selected_event_id)
                .cloned()
        });
        let selected = if let Some(event) = selected {
            let selected_id = event.id;
            Some((selected_id, event))
        } else {
            None
        };

        let filter_query = serde_urlencoded::to_string(query.filter.clone()).unwrap();
        context.insert("filter_query", &filter_query);

        context.insert("events", &events);
        context.insert("available_tags", &available_tags);
        context.insert("events_grouped_by_priority", &events_grouped_by_priority);
        context.insert("selected_id", &selected.clone().map(|(id, _)| id));
        if selected.is_none() {
            if let Some(occurrence_id) = query.from_occurrence {
                let matching =
                    get_visible_event_occurrence_with_event(&data, Some(user.id), occurrence_id)
                        .await
                        .or_any_internal_server_error(
                            "Failed to fetch requested event occurrence for local events page",
                        )?;
                if let Some((event, occurrence)) = matching {
                    let as_local = LocalEventForm::from(LocalEvent {
                        user_id: user.id,
                        starts_at: occurrence.starts_at,
                        description: event.description,
                        all_day: event.all_day,
                        summary: event.summary,
                        priority: event.priority,
                        duration: event.duration,
                        location: event.location,
                        created_at: get_current_time(),
                        updated_at: get_current_time(),
                        bill: None,
                        attendance_planned: true,
                        attendance_actual: false,
                        tags: vec![],
                        uid: "".to_owned(),
                        id: -1,
                    });
                    context.insert("selected", &as_local);
                    context.insert("linked_occurrence_id", &occurrence.id);
                }
            }
        } else {
            context.insert(
                "selected",
                &selected.map(|(_, event)| LocalEventForm::from(event)),
            );
        }

        let content = data.templates.render("pages/local.html", &context).unwrap();
        return Ok(remove_flash_cookie(HttpResponse::Ok()).body(content));
    }
    Ok(redirect("/me").finish())
}

#[get("/remote/sources/{id}")]
async fn source(
    data: web::Data<AppState>,
    path: web::Path<i32>,
    request: HttpRequest,
) -> impl Responder {
    let (mut context, user, _key) = request.get_session_context(&data).await;
    let id = path.into_inner();
    let (source, events, occurrences) =
        get_source_as_user_with_event_count(&data, user.map(|u| u.id), id).await;
    context.insert("source", &source);
    context.insert("event_count", &events);
    context.insert("occurrence_count", &occurrences);
    let content = data
        .templates
        .render("pages/source.html", &context)
        .unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}

#[get("/admin")]
async fn admin(data: web::Data<AppState>, request: HttpRequest) -> impl Responder {
    let (mut context, user, _key) = request.get_session_context(&data).await;
    if let Some(response) = admin_check(user) {
        return response;
    }
    let users = sqlx::query_as!(RawUser, "SELECT * FROM users")
        .fetch_all(&data.conn)
        .await
        .expect("Failed to get users");
    let users = users.into_iter().map(UserPublic::from).collect::<Vec<_>>();
    context.insert("users", &users);
    let unverified_users = sqlx::query_as!(UnverifiedUser, "SELECT * FROM unverified_users")
        .fetch_all(&data.conn)
        .await
        .expect("Failed to get unverified users");
    context.insert("unverified_users", &unverified_users);
    let content = data.templates.render("pages/admin.html", &context).unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}

#[get("/me")]
pub async fn me(
    data: web::Data<AppState>,
    request: HttpRequest,
) -> Result<impl Responder, InternalServerError<sqlx::Error>> {
    let (mut context, user, _key) = request.get_session_context(&data).await;
    let mut greeting = "Welcome";
    if let Some(user) = user {
        context.insert(
            "export_links",
            &get_user_export_links(&data, user.id).await?,
        );

        let api_keys = sqlx::query_as!(
            RawApiKey,
            "SELECT * FROM api_keys WHERE user_id = $1",
            user.id
        )
        .fetch_all(&data.conn)
        .await
        .or_internal_server_error("Failed to fetch api keys for /me")?
        .into_iter()
        .map(|raw| ApiKey::try_from(raw).map(ApiKeyForm::from))
        .collect::<Result<Vec<_>, _>>()
        .expect("invalid api keys returned from db for /me");
        context.insert("api_keys", &api_keys);

        let all_timezones = chrono_tz::TZ_VARIANTS
            .iter()
            .map(|tz| tz.name())
            .collect::<Vec<_>>();
        context.insert("timezones", &all_timezones);

        let user_local_time = user.get_current_local_time();
        let hm = user_local_time.hour() * 100 + user_local_time.minute();
        greeting = match hm {
            0..500 => "Sleep tight",
            500..1230 => "Good morning",
            1230..1630 => "Good afternoon",
            1630..2100 => "Good evening",
            2100..2400 => "Good night",
            2400.. => {
                panic!("Greeting: Invalid time: {hm}");
            }
        };
    }
    context.insert("greeting", &greeting);

    let content = data.templates.render("pages/me.html", &context).unwrap();
    Ok(remove_flash_cookie(HttpResponse::Ok()).body(content))
}

#[derive(Debug, serde::Deserialize)]
struct IndexQuery {
    year: Option<i32>,
    month: Option<u32>,
    min_priority: Option<Priority>,
    max_priority: Option<Priority>,
}
#[get("/list")]
async fn list(
    data: web::Data<AppState>,
    request: HttpRequest,
    filter: Query<IndexQuery>,
) -> impl Responder {
    let (mut context, user, _key) = request.get_session_context(&data).await;
    if let Some(user) = user {
        let pivot = if let Some(month) = filter.month {
            if let Some(year) = filter.year {
                chrono::NaiveDate::from_ymd_opt(year, month, 1)
                    .expect("Failed to construct pivot")
                    .and_time(NaiveTime::MIN)
                    .and_utc()
            } else {
                let year = chrono::Utc::now().year();
                chrono::NaiveDate::from_ymd_opt(year, month, 1)
                    .expect("Failed to construct pivot")
                    .and_time(NaiveTime::MIN)
                    .and_utc()
            }
        } else {
            chrono::Utc::now()
        };
        // after yesterday (from today)
        let yesterday = (pivot - chrono::Duration::days(1)).timestamp();
        // before next month
        let next_month = (pivot + chrono::Duration::days(30)).timestamp();
        let events = get_visible_event_occurrences(
            &data,
            Some(user.id),
            true,
            &EventFilter {
                after: Some(yesterday),
                before: Some(next_month),
                min_priority: filter.min_priority,
                max_priority: filter.max_priority,
                ..Default::default()
            },
        )
        .await;
        // humanize dates etc
        let events = events
            .into_iter()
            .map(|e| EventOccurrenceHuman::from((e, &user.interface_timezone_parsed)))
            .collect::<Vec<_>>();

        context.insert("events", &events);

        // generate data for the year and month selectors
        let years = (pivot.year() - 4..=pivot.year() + 6).collect::<Vec<_>>();
        let months = (1..=12).collect::<Vec<_>>();
        context.insert("years", &years);
        context.insert("months", &months);
        context.insert("selected_year", &pivot.year());
        context.insert("selected_month", &pivot.month());
    }

    let content = data.templates.render("pages/list.html", &context).unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}

#[derive(Debug, PartialEq)]
struct CalendarPosition {
    year: i32,
    week: u32,
}
#[derive(Debug, serde::Deserialize, PartialEq)]
struct CalendarPositionRaw {
    year: String,
    week: String,
}
impl TryFrom<CalendarPositionRaw> for CalendarPosition {
    type Error = ();
    fn try_from(raw: CalendarPositionRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            year: raw.year.parse().map_err(|_| ())?,
            week: raw.week.parse().map_err(|_| ())?,
        })
    }
}
#[derive(Debug, serde::Deserialize, PartialEq)]
struct CalendarPositionGoto {
    goto: String,
}

use serde_with::rust::deserialize_ignore_any;

use crate::db_utils::{
    events::{
        get_user_local_events, get_visible_event_occurrence_with_event,
        get_visible_event_occurrences,
    },
    request::{
        deauth, redirect, AnyInternalServerError, EnhancedRequest, InternalServerError,
        OrInternalServerError,
    },
    sources::{get_source_as_user_with_event_count, get_visible_sources_with_event_count},
    timeline::compile_timeline,
    user::get_user_export_links,
};
#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(untagged)]
enum CalendarQueryPosition {
    Position(CalendarPositionRaw),
    Goto(CalendarPositionGoto),
    #[serde(deserialize_with = "deserialize_ignore_any")]
    NotPresent,
}

fn parse_goto(goto: &str) -> Option<CalendarPosition> {
    if let Some(ts_str) = goto.strip_prefix('t') {
        let timestamp: i64 = ts_str.parse().ok()?;
        let date = from_timestamp(timestamp);
        return Some(CalendarPosition {
            year: date.year(),
            week: date.iso_week().week(),
        });
    }
    let mut year = None;
    let mut buffer = String::new();
    let mut mode_week_or_mmdd = false; // week = true, mmdd = false
    for ch in goto.chars() {
        match year {
            None => {
                buffer.push(ch);
                if buffer.len() > 3 {
                    year = Some(buffer.parse().ok()?);
                    buffer.clear();
                }
            }
            Some(_) => match (ch, buffer.len()) {
                ('w', 0) => {
                    mode_week_or_mmdd = true;
                }
                _ => {
                    buffer.push(ch);
                }
            },
        }
    }

    if year.is_none() {
        year = Some(buffer.parse().ok()?);
        buffer.clear();
    }
    let year = year.unwrap();

    // no week or mmdd
    if buffer.is_empty() {
        return Some(CalendarPosition { year, week: 1 });
    }

    let week = if mode_week_or_mmdd {
        buffer.parse().ok()?
    } else {
        let (month, day) = match buffer.len() {
            4 => (buffer[..2].parse().ok()?, buffer[2..].parse().ok()?),
            2 => (buffer.parse().ok()?, 1),
            _ => return None,
        };
        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
        date.iso_week().week()
    };

    // a year might contain at most 53 weeks
    if week > 53 {
        return None;
    }

    Some(CalendarPosition { year, week })
}
impl From<CalendarQueryPosition> for Option<CalendarPosition> {
    fn from(position: CalendarQueryPosition) -> Self {
        match position {
            CalendarQueryPosition::Position(position) => CalendarPosition::try_from(position).ok(),
            CalendarQueryPosition::Goto(g) => parse_goto(&g.goto),
            _ => None,
        }
    }
}

const INTERFACE_MIN_EVENT_LENGTH: i32 = 3600; // 1 hour

#[derive(Debug, serde::Deserialize, PartialEq)]
struct CalendarQuery {
    #[serde(flatten)]
    position: CalendarQueryPosition,
    #[serde(flatten)]
    filter: RawEventFilter,
}
#[get("/")]
async fn calendar(
    data: web::Data<AppState>,
    request: HttpRequest,
    query: Query<CalendarQuery>,
) -> impl Responder {
    let (mut context, user, _key) = request.get_session_context(&data).await;
    if let Some(user) = user {
        let now = chrono::Utc::now().with_timezone(&user.interface_timezone_parsed);

        context.insert("filter", &query.filter);
        context.insert("filter_set", &query.filter.is_defined());
        let query = query.into_inner();
        let chosen_position: Option<CalendarPosition> = query.position.into();
        let mut filter = EventFilter::from(query.filter);
        // pivot is the first day of the shown week at 00:00 UTC
        let pivot = if let Some(position) = chosen_position {
            chrono::NaiveDate::from_isoywd_opt(position.year, position.week, chrono::Weekday::Mon)
                .expect("Failed to construct pivot")
                .and_time(NaiveTime::MIN)
                .and_utc()
        } else {
            let year = now.year();
            // get monday of the current week
            let week = now.iso_week().week();
            chrono::NaiveDate::from_isoywd_opt(year, week, chrono::Weekday::Mon)
                .expect("Failed to construct pivot")
                .and_time(NaiveTime::MIN)
                .and_utc()
        };
        let pivot_local = pivot
            .with_timezone(&user.interface_timezone_parsed)
            .with_time(NaiveTime::MIN)
            .earliest()
            .expect("Failed to convert pivot to local time");

        // after yesterday (from today)
        let from = (pivot - chrono::Duration::milliseconds(1)).timestamp();
        // before next week
        let to = (pivot + chrono::Duration::days(7)).timestamp();
        filter.after = Some(from);
        filter.before = Some(to);
        let events = get_visible_event_occurrences(&data, Some(user.id), true, &filter).await;
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
            .map(|e| EventOccurrenceHuman::from((e, &user.interface_timezone_parsed)))
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
        context.insert("events", &events);

        let current_day: Option<usize> = if now.iso_week() == pivot.iso_week() {
            Some(now.weekday().number_from_monday() as usize - 1)
        } else {
            None
        };
        context.insert("current_day", &current_day);
        let current_time = now.time();
        let current_time_seconds = current_time.hour() * 3600 + current_time.minute() * 60;
        context.insert("current_time_seconds", &current_time_seconds);
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
        context.insert("events_by_day", &events_by_day);

        // generate data for the year and month selectors
        let before = pivot - chrono::Duration::weeks(1);
        let after = pivot + chrono::Duration::weeks(1);
        let week_before = before.iso_week();
        let week_after = after.iso_week();
        let year_before = week_before.year();
        let year_after = week_after.year();
        context.insert("prev_year", &year_before);
        context.insert("prev_week", &week_before.week());
        context.insert("next_year", &year_after);
        context.insert("next_week", &week_after.week());
        // generate dates for the current week
        let mut week_dates = vec![];
        for (i, day) in (0..7)
            .map(|i| pivot + chrono::Duration::days(i))
            .enumerate()
        {
            let formatted = day.format("%d.%-m.").to_string();
            week_dates.push(formatted.clone());
            context.insert(format!("week_date_{}", i), &formatted);
        }
        context.insert("week_dates", &week_dates);
        context.insert(
            "day_names",
            &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
        );

        context.insert("selected_year", &pivot.iso_week().year());
        context.insert("selected_week", &pivot.iso_week().week());
        let content = data
            .templates
            .render("pages/calendar.html", &context)
            .unwrap();
        return remove_flash_cookie(HttpResponse::Ok()).body(content);
    }

    let content = data.templates.render("pages/index.html", &context).unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}

#[derive(Debug, serde::Deserialize)]
struct TimelineQuery {
    #[serde(flatten)]
    filter: RawEventFilterWithDate,
    #[serde(default)]
    granularity: TimelineGranularity,
}
#[derive(Debug, serde::Deserialize)]
pub enum TimelineGranularity {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Second,
}
impl Default for TimelineGranularity {
    fn default() -> Self {
        Self::Week
    }
}
#[get("/timeline")]
pub async fn timeline(
    data: web::Data<AppState>,
    request: HttpRequest,
    query: web::Query<TimelineQuery>,
) -> impl Responder {
    let (mut context, user_opt, _key) = request.get_session_context(&data).await;
    if let Some(user) = user_opt {
        tracing::info!(user.id, user.email, "User requested timeline");

        const SECONDS_IN_HOUR: i64 = 60 * 60;
        const SECONDS_IN_DAY: i64 = 24 * SECONDS_IN_HOUR;
        const SECONDS_IN_WEEK: i64 = 7 * SECONDS_IN_DAY;
        const SECONDS_IN_MONTH: i64 = 30 * SECONDS_IN_DAY;
        const SECONDS_IN_YEAR: i64 = 365 * SECONDS_IN_DAY;
        let granularity_seconds = match query.granularity {
            TimelineGranularity::Year => SECONDS_IN_YEAR,
            TimelineGranularity::Month => SECONDS_IN_MONTH,
            TimelineGranularity::Week => SECONDS_IN_WEEK,
            TimelineGranularity::Day => SECONDS_IN_DAY,
            TimelineGranularity::Hour => SECONDS_IN_HOUR,
            TimelineGranularity::Second => 1,
        };

        let filter = EventFilter::from(query.filter.clone());
        context.insert("filter", &query.filter);
        context.insert("filter_set", &query.filter.is_defined());
        let timeline = compile_timeline(&data, user.id, &filter, granularity_seconds)
            .await
            .expect("Failed to compile timeline!");

        context.insert("timeline", &timeline);

        let mut years = vec![];

        if let (Some(min_chunk), Some(max_chunk)) = (timeline.min_date, timeline.max_date) {
            let total_chunks = max_chunk - min_chunk;
            let seconds_in_year = 60 * 60 * 24 * 365;
            let chunk_size = timeline.chunk_size;
            let chunks_in_year = seconds_in_year / chunk_size;

            let min_chunk_snapped_to_year = min_chunk - (min_chunk % chunks_in_year);
            for year in 0..=total_chunks / chunks_in_year {
                let year_start: i64 = min_chunk_snapped_to_year + year * chunks_in_year;
                let year_since_zero = 1970 + (year_start / chunks_in_year);
                years.push((year_start, year_since_zero));
            }
        }
        context.insert("timeline_years", &years);

        let content = data
            .templates
            .render("pages/timeline.html", &context)
            .unwrap();
        return HttpResponse::Ok().body(content);
    }
    deauth(&request)
}

pub fn routes() -> Scope {
    web::scope("")
        .service(sources)
        .service(local)
        .service(source)
        .service(me)
        .service(list)
        .service(calendar)
        .service(timeline)
        .service(admin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_urlencoded::from_str;

    #[test]
    fn test_calendar_query_empty() {
        let query = "";
        let query: CalendarQuery = from_str(query).unwrap();
        assert_eq!(query.position, CalendarQueryPosition::NotPresent);
        assert_eq!(query.filter, RawEventFilter::default());
    }

    #[test]
    fn test_calendar_query() {
        let query = "year=2021&week=1&min_priority=1&max_priority=2";
        let query: CalendarQuery = from_str(query).unwrap();
        assert_eq!(
            query.position,
            CalendarQueryPosition::Position(CalendarPositionRaw {
                year: "2021".to_string(),
                week: "1".to_string()
            })
        );
        assert_eq!(
            query.filter,
            RawEventFilter {
                min_priority: Some("1".to_string()),
                max_priority: Some("2".to_string()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_calendar_goto_year() {
        let query = "goto=2021";
        let query: CalendarQuery = from_str(query).unwrap();
        assert_eq!(
            query.position,
            CalendarQueryPosition::Goto(CalendarPositionGoto {
                goto: "2021".to_string()
            })
        );
        assert_eq!(query.filter, RawEventFilter::default());
    }
}

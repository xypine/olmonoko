mod utils;


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
        api_key::{ApiKey, ApiKeyForm, RawApiKey}, event::{
            local::{LocalEvent, LocalEventForm, LocalEventId},
            remote::RemoteEventOccurrenceId,
            EventOccurrenceHuman, Priority,
        }, public_link::{PublicLink, RawPublicLink}, user::{RawUser, TimezoneEntity, UnverifiedUser, User, UserPublic}
    },
    utils::{
        event_filters::{EventFilter, RawEventFilter, RawEventFilterWithDate},
        flash::FLASH_COOKIE_NAME,
        time::{from_timestamp, get_current_time},
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
use utils::build_calendar;
use uuid::Uuid;

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
        let calendar_data = build_calendar(&data, user.id, pivot, user.interface_timezone_parsed, EventFilter::from(query.filter)).await;
        calendar_data.insert_into_context(&mut context);
        let content = data
            .templates
            .render("pages/calendar.html", &context)
            .unwrap();
        return remove_flash_cookie(HttpResponse::Ok()).body(content);
    }

    let content = data.templates.render("pages/index.html", &context).unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}
#[get("/shared/{id}")]
async fn calendar_shared(
    data: web::Data<AppState>,
    request: HttpRequest,
    query: Query<CalendarQuery>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, InternalServerError<sqlx::Error>> {
    let (mut context, user, _key) = request.get_session_context(&data).await;
    let id = path.into_inner().to_string();
    context.insert("link_id", &id);
    tracing::info!("Fetching calendar for id {id}");
    let opt = sqlx::query!(
        "SELECT public_calendar_links.*, users.interface_timezone, users.email, users.admin, users.created_at AS user_created_at FROM public_calendar_links INNER JOIN users ON public_calendar_links.user_id = users.id WHERE public_calendar_links.id = $1",
        id
    )
    .fetch_optional(&data.conn)
    .await
    .or_internal_server_error("Failed to fetch public calendar link from the database")?
    .map(|row| {
        let link = PublicLink::from(RawPublicLink{
            user_id: row.user_id,
            created_at: row.created_at,
            id: row.id,
            min_priority: row.min_priority,
            max_priority: row.max_priority,
        });
        let user = UserPublic::from(User::from(RawUser{
            id: row.user_id,
            interface_timezone: row.interface_timezone,
            email: row.email,
            password_hash: "INTENTIONALLY_NOT_FILLED".to_string(),
            created_at: row.user_created_at,
            admin: row.admin
        }));

        (link, user)
    });
    if let Some((link, link_owner)) = opt {
        context.insert("link_owner", &link_owner);

        let tz = user.map(|u| u.interface_timezone_parsed).unwrap_or(link_owner.interface_timezone_parsed);
        let now = chrono::Utc::now().with_timezone(&tz);

        context.insert("filter", &query.filter);
        context.insert("filter_set", &query.filter.is_defined());
        let query = query.into_inner();
        let chosen_position: Option<CalendarPosition> = query.position.into();
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
        let mut filter = EventFilter::from(query.filter);
        filter.min_priority = link.min_priority;
        filter.max_priority = link.max_priority;
        let calendar_data = build_calendar(&data, link.user_id, pivot, tz, filter).await;
        calendar_data.insert_into_context(&mut context);
        let content = data
            .templates
            .render("pages/calendar.html", &context)
            .unwrap();
        return Ok(remove_flash_cookie(HttpResponse::Ok()).body(content));
    }

    let content = data.templates.render("pages/index.html", &context).unwrap();
    Ok(remove_flash_cookie(HttpResponse::Ok()).body(content))
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
        .service(calendar_shared)
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

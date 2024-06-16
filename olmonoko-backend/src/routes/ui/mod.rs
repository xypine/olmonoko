use crate::{
    models::{
        bills::RawBill,
        event::{
            local::{LocalEvent, LocalEventForm, RawLocalEvent},
            remote::{RawRemoteEvent, RemoteEvent},
            Event, EventOccurrence, EventOccurrenceHuman, DEFAULT_PRIORITY, PRIORITY_OPTIONS,
        },
        user::{RawUser, UserPublic},
    },
    routes::{
        api::{
            data_source::{get_source_as_user, get_visible_sources_with_event_count},
            export::get_user_export_links,
            user::{get_user_from_request, redirect},
        },
        AppState,
    },
    utils::flash::{FlashMessage, FLASH_COOKIE_NAME},
};
use actix_web::{
    cookie::SameSite,
    get,
    web::{self, Query},
    HttpRequest, HttpResponse, HttpResponseBuilder, Responder, Scope,
};
use chrono::{Datelike, NaiveTime, Timelike};
use itertools::Itertools;

pub(crate) async fn get_session_context(
    data: &web::Data<AppState>,
    request: &HttpRequest,
) -> (tera::Context, Option<UserPublic>) {
    let flash_message = request
        .cookie(FLASH_COOKIE_NAME)
        .map(|c| FlashMessage::from_cookie(&c));
    let user = get_user_from_request(data, request)
        .await
        .map(UserPublic::from);
    let path = request.path();
    let root_path = request.path().split('/').nth(1).unwrap_or("");
    let mut context = tera::Context::new();
    context.insert("site_url", &data.site_url);
    context.insert("path", &path);
    context.insert("root_path", &root_path);
    context.insert("version", &data.version);
    context.insert("flash", &flash_message);
    context.insert("user", &user);
    context.insert("event_priority_options", &PRIORITY_OPTIONS);
    (context, user)
}

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
    let (mut context, user) = get_session_context(&data, &request).await;
    let all_sources = get_visible_sources_with_event_count(&data, user.map(|u| u.id)).await;
    context.insert("sources", &all_sources);

    let content = data
        .templates
        .render("pages/sources.html", &context)
        .unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct EventFilter {
    pub summary_like: Option<String>,
    pub after: Option<i64>,
    pub before: Option<i64>,
    pub min_priority: Option<i64>,
    pub max_priority: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct RawEventFilter {
    pub summary_like: Option<String>,
    pub min_priority: Option<String>,
    pub max_priority: Option<String>,
    pub tags: Option<String>,
    pub exclude_tags: Option<String>,
}
impl From<RawEventFilter> for EventFilter {
    fn from(raw: RawEventFilter) -> Self {
        Self {
            summary_like: raw.summary_like,
            after: None,
            before: None,
            min_priority: raw.min_priority.and_then(|s| s.parse().ok()),
            max_priority: raw.max_priority.and_then(|s| s.parse().ok()),
            tags: raw.tags.map(|s| {
                s.split(',')
                    .map(str::to_string)
                    .filter(|s| !s.is_empty())
                    .collect()
            }),
            exclude_tags: raw.exclude_tags.map(|s| {
                s.split(',')
                    .map(str::to_string)
                    .filter(|s| !s.is_empty())
                    .collect()
            }),
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RawEventDateFilter {
    pub after: Option<String>,
    pub before: Option<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RawEventFilterWithDate {
    #[serde(flatten)]
    pub base: RawEventFilter,
    #[serde(flatten)]
    pub date: RawEventDateFilter,
}
impl From<RawEventFilterWithDate> for EventFilter {
    fn from(raw: RawEventFilterWithDate) -> Self {
        let mut base = EventFilter::from(raw.base);
        let after = raw.date.after.and_then(|s| s.parse().ok());
        let before = raw.date.before.and_then(|s| s.parse().ok());
        base.after = after;
        base.before = before;
        base
    }
}

#[derive(Debug, serde::Deserialize)]
struct LocalQuery {
    selected: Option<i64>,
    #[serde(flatten)]
    filter: RawEventFilterWithDate,
}
#[get("/local")]
async fn local(
    data: web::Data<AppState>,
    request: HttpRequest,
    query: Query<LocalQuery>,
) -> impl Responder {
    let (mut context, user) = get_session_context(&data, &request).await;
    if let Some(user) = user {
        println!("query: {:?}", query);
        let filter = EventFilter::from(query.filter.clone());
        let events = get_user_local_events(&data, user.id, false, filter.clone()).await;
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
            .group_by(|event| event.priority)
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
                .map(LocalEventForm::from)
                .map(|form| (selected_event_id, form))
        });

        // TODO: Fix this mess
        context.insert("filter", &filter);
        // println!("filter: '{:?}'", filter);
        let filter_query = serde_urlencoded::to_string(query.filter.clone()).unwrap();
        context.insert("filter_query", &filter_query);
        // println!("filter_query: '{}'", filter_query);

        context.insert("events", &events);
        context.insert("available_tags", &available_tags);
        context.insert("events_grouped_by_priority", &events_grouped_by_priority);
        context.insert("selected_id", &selected.clone().map(|(id, _)| id));
        context.insert("selected", &selected.map(|(_, form)| form));
        let content = data.templates.render("pages/local.html", &context).unwrap();
        return remove_flash_cookie(HttpResponse::Ok()).body(content);
    }
    redirect("/me").finish()
}

#[get("/remote/sources/{id}")]
async fn source(
    data: web::Data<AppState>,
    path: web::Path<i32>,
    request: HttpRequest,
) -> impl Responder {
    let (mut context, user) = get_session_context(&data, &request).await;
    let id = path.into_inner();
    let source = get_source_as_user(&data, user.map(|u| u.id), id).await;
    context.insert("source", &source);
    let content = data
        .templates
        .render("pages/source.html", &context)
        .unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}

#[get("/admin")]
async fn admin(data: web::Data<AppState>, request: HttpRequest) -> impl Responder {
    let (mut context, user) = get_session_context(&data, &request).await;
    if let Some(response) = admin_check(user) {
        return response;
    }
    let users = sqlx::query_as!(RawUser, "SELECT * FROM users")
        .fetch_all(&data.conn)
        .await
        .expect("Failed to get users");
    let users = users.into_iter().map(UserPublic::from).collect::<Vec<_>>();
    context.insert("users", &users);
    let content = data.templates.render("pages/admin.html", &context).unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}

#[get("/me")]
async fn me(data: web::Data<AppState>, request: HttpRequest) -> impl Responder {
    let (mut context, user) = get_session_context(&data, &request).await;
    if let Some(user) = user {
        context.insert("export_links", &get_user_export_links(&data, user.id).await);

        let all_timezones = chrono_tz::TZ_VARIANTS
            .iter()
            .map(|tz| tz.name())
            .collect::<Vec<_>>();
        context.insert("timezones", &all_timezones);
    }

    let content = data.templates.render("pages/me.html", &context).unwrap();
    remove_flash_cookie(HttpResponse::Ok()).body(content)
}

pub async fn get_user_local_events(
    data: &web::Data<AppState>,
    user_id: i64,
    autodescription: bool,
    filter: EventFilter,
) -> Vec<LocalEvent> {
    let min_priority = parse_priority(filter.min_priority);
    let max_priority = parse_priority(filter.max_priority);
    let tags = filter.tags.map(|tags| tags.join(","));
    let exclude_tags = filter.exclude_tags.map(|tags| tags.join(","));
    sqlx::query!(
        r#"
        SELECT event.*, 
            bill.id as "bill_id?", 
            bill.payee_account_number, 
            bill.reference, 
            bill.amount, 
            bill.created_at as bill_created_at, 
            bill.updated_at as bill_updated_at,
            bill.payee_name,
            bill.payee_email,
            bill.payee_address,
            bill.payee_phone,
            GROUP_CONCAT(tag.tag, ',') AS tags
        FROM local_events AS event
        LEFT JOIN bills AS bill 
            ON bill.local_event_id = event.id 
        LEFT JOIN event_tags AS tag 
            ON tag.local_event_id = event.id
        WHERE event.user_id = $1 
            AND ($2 IS NULL OR event.starts_at > $2) 
            AND ($3 IS NULL OR event.starts_at < $3) 
            AND (COALESCE(NULLIF(event.priority, 0), $6) >= $4 OR $4 IS NULL)
            AND (COALESCE(NULLIF(event.priority, 0), $6) <= $5 OR $5 IS NULL)
            AND ($7 IS NULL OR event.summary LIKE $7)
            AND ($8 IS NULL OR tag.tag IN ($8))
            AND ($9 IS NULL OR tag IS NULL OR (
                SELECT tag.tag
                FROM event_tags AS tag
                WHERE tag.local_event_id = event.id
                AND tag.tag IN ($9)
            ) IS NULL)
        GROUP BY event.id
        ORDER BY event.starts_at;
        "#,
        user_id,
        filter.after,
        filter.before,
        min_priority,
        max_priority,
        DEFAULT_PRIORITY,
        filter.summary_like,
        tags,
        exclude_tags,
    )
    .fetch_all(&data.conn)
    .await
    .expect("Failed to get events")
    .into_iter()
    .map(|event| {
        let raw_event = RawLocalEvent {
            id: event.id,
            user_id,
            created_at: event.created_at,
            updated_at: event.updated_at,
            priority: event.priority,
            starts_at: event.starts_at,
            all_day: event.all_day,
            uid: event.uid,
            summary: event.summary,
            duration: event.duration,
            location: event.location,
            description: event.description,
        };
        let raw_bill = event.bill_id.map(|bill_id| RawBill {
            id: bill_id,
            local_event_id: Some(event.id),
            remote_event_id: None,
            payee_account_number: event
                .payee_account_number
                .expect("Missing bill_payee_account_number"),
            reference: event.reference.expect("Missing bill_reference"),
            amount: event.amount.expect("Missing bill_amount"),
            created_at: event.bill_created_at.expect("Missing bill_created_at"),
            updated_at: event.bill_updated_at.expect("Missing bill_updated_at"),
            payee_name: event.payee_name,
            payee_email: event.payee_email,
            payee_address: event.payee_address,
            payee_phone: event.payee_phone,
        });
        let tags = event.tags.unwrap_or_default();
        LocalEvent::from((raw_event, raw_bill, autodescription, tags.as_str()))
    })
    .collect()
}

pub fn parse_priority(priority: Option<i64>) -> Option<i64> {
    if let Some(priority) = priority {
        if priority == 0 {
            Some(DEFAULT_PRIORITY)
        } else {
            Some(priority)
        }
    } else {
        None
    }
}

async fn get_visible_remote_events(
    data: &web::Data<AppState>,
    user_id: Option<i64>,
    after: Option<i64>,
    before: Option<i64>,
    min_priority: Option<i64>,
    max_priority: Option<i64>,
) -> Vec<(RemoteEvent, i64, bool)> {
    let min_priority = parse_priority(min_priority);
    let max_priority = parse_priority(max_priority);
    sqlx::query!(
        r#"
        SELECT 
            e.*, 
            p.priority, 
            o.starts_at, 
            o.from_rrule 
        FROM 
            events AS e 
        INNER JOIN 
            ics_sources AS s 
            ON e.event_source_id = s.id 
            AND (s.user_id = $1 OR s.is_public)
        INNER JOIN 
            event_occurrences AS o 
            ON o.event_id = e.id 
        INNER JOIN 
            ics_source_priorities AS p 
            ON p.user_id = $1 
            AND p.ics_source_id = s.id 
            -- min_priority is null or (source_in_calendar and event_priority_override >= min_priority) or source_priority >= min_priority
            AND ($4 IS NULL OR (p.priority IS NOT NULL AND COALESCE(NULLIF(e.priority_override, 0), $6) >= $4) OR COALESCE(NULLIF(p.priority, 0), $6) >= $4)
            -- max_priority is null or (source_in_calendar and event_priority_override <= max_priority) and source_priority <= max_priority
            AND ($5 IS NULL OR (p.priority IS NOT NULL AND COALESCE(NULLIF(e.priority_override, 0), $6) <= $5) AND COALESCE(NULLIF(p.priority, 0), $6) <= $5)
        WHERE 
            ($2 IS NULL OR o.starts_at > $2) 
            AND ($3 IS NULL OR o.starts_at < $3) 
        ORDER BY 
            o.starts_at;
        "#,
        user_id,
        after,
        before,
        min_priority,
        max_priority,
        DEFAULT_PRIORITY,
    )
    .fetch_all(&data.conn)
    .await
    .expect("Failed to get events")
    .into_iter()
    .map(|event| {
        (
            RemoteEvent::from(RawRemoteEvent {
                priority_override: event.priority_override,
                rrule: event.rrule,
                id: event.id,
                event_source_id: event.event_source_id,
                event_source_priority: event.priority,
                uid: event.uid,
                summary: event.summary,
                dt_stamp: event.dt_stamp,
                all_day: event.all_day,
                duration: event.duration,
                location: event.location,
                description: event.description,
            }),
            event.starts_at,
            event.from_rrule,
        )
    })
    .collect()
}
pub async fn get_visible_events(
    data: &web::Data<AppState>,
    user_id: Option<i64>,
    autodescription: bool,
    after: Option<i64>,
    before: Option<i64>,
    min_priority: Option<i64>,
    max_priority: Option<i64>,
) -> Vec<Event> {
    // remote
    let remote_events =
        get_visible_remote_events(data, user_id, after, before, min_priority, max_priority).await;
    let mut events: Vec<Event> = remote_events
        .into_iter()
        .sorted_by_key(|(event, _, _)| event.id)
        .group_by(|(event, _, _)| event.id)
        .into_iter()
        .flat_map(|(_, group)| {
            let group: Vec<_> = group.collect();
            if group.is_empty() {
                None
            } else {
                let (event, _, _) = group.first().unwrap().clone();
                let starts_at = group
                    .into_iter()
                    .map(|(_, starts_at, _)| starts_at)
                    .collect::<Vec<_>>();
                Some((event, starts_at))
            }
        })
        .map(Event::from)
        .collect();
    if let Some(user_id) = user_id {
        let local_events: Vec<Event> = get_user_local_events(
            data,
            user_id,
            autodescription,
            EventFilter {
                summary_like: None,
                after,
                before,
                min_priority,
                max_priority,
                ..Default::default()
            },
        )
        .await
        .into_iter()
        .map(Event::from)
        .collect();
        events.extend(local_events);
    }
    events.sort_by_key(|event| {
        event
            .starts_at
            .first()
            .expect("event missing any occurrence after aggregation")
            .timestamp()
    });
    events
}
pub async fn get_visible_event_occurrences(
    data: &web::Data<AppState>,
    user_id: Option<i64>,
    autodescription: bool,
    after: Option<i64>,
    before: Option<i64>,
    min_priority: Option<i64>,
    max_priority: Option<i64>,
) -> Vec<EventOccurrence> {
    let events = get_visible_events(
        data,
        user_id,
        autodescription,
        after,
        before,
        min_priority,
        max_priority,
    )
    .await;
    events
        .into_iter()
        .flat_map(Vec::<EventOccurrence>::from)
        .sorted_by_key(|event| event.starts_at.timestamp())
        .collect()
}

#[derive(Debug, serde::Deserialize)]
struct IndexQuery {
    year: Option<i32>,
    month: Option<u32>,
    min_priority: Option<i64>,
    max_priority: Option<i64>,
}
#[get("/list")]
async fn list(
    data: web::Data<AppState>,
    request: HttpRequest,
    filter: Query<IndexQuery>,
) -> impl Responder {
    let (mut context, user) = get_session_context(&data, &request).await;
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
            Some(yesterday),
            Some(next_month),
            filter.min_priority,
            filter.max_priority,
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
#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(untagged)]
enum CalendarQueryPosition {
    Position(CalendarPositionRaw),
    Goto(CalendarPositionGoto),
    #[serde(deserialize_with = "deserialize_ignore_any")]
    NotPresent,
}

fn parse_goto(goto: &str) -> Option<CalendarPosition> {
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

#[derive(Debug, serde::Deserialize, PartialEq)]
struct CalendarQuery {
    #[serde(flatten)]
    position: CalendarQueryPosition,
    #[serde(flatten)]
    filter: RawEventFilter,
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
#[get("/")]
async fn calendar(
    data: web::Data<AppState>,
    request: HttpRequest,
    query: Query<CalendarQuery>,
) -> impl Responder {
    let (mut context, user) = get_session_context(&data, &request).await;
    if let Some(user) = user {
        let now = chrono::Utc::now().with_timezone(&user.interface_timezone_parsed);

        let query = query.into_inner();
        let chosen_position: Option<CalendarPosition> = query.position.into();
        let filter = EventFilter::from(query.filter);
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
        // after yesterday (from today)
        let yesterday = (pivot - chrono::Duration::days(1)).timestamp();
        // before next week
        let next_week = (pivot + chrono::Duration::days(7)).timestamp();
        let events = get_visible_event_occurrences(
            &data,
            Some(user.id),
            true,
            Some(yesterday),
            Some(next_week),
            filter.min_priority,
            filter.max_priority,
        )
        .await;
        // humanize dates etc
        let events = events
            .into_iter()
            .map(|e| EventOccurrenceHuman::from((e, &user.interface_timezone_parsed)))
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
                .filter(|event| event.starts_at.weekday().number_from_monday() - 1 == day)
                .cloned()
                .sorted_by_key(|event| event.priority)
                .collect::<Vec<_>>();

            let is_today = current_day == Some(day as usize);
            if is_today {
                day_events.push(EventOccurrenceHuman {
                    id: -1,
                    source: crate::models::event::EventSource::Local(
                        crate::models::event::SourceLocal { user_id: -1 },
                    ),
                    priority: 1,
                    starts_at: now,
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
                })
            }

            for event in &mut day_events {
                // set event duration to be at least 1 hour
                if event.duration.unwrap_or(0) < 3600 {
                    event.duration = Some(3600);
                }
            }

            // find overlapping events and adjust event overlap_count and overlap_index
            let events_len = day_events.len();
            for i in 0..events_len {
                let mut overlap_total = 1;
                let mut overlap_index = 0;
                for j in 0..events_len {
                    if i == j {
                        continue;
                    }
                    let event = &day_events[i];
                    let other = &day_events[j];
                    if (event.starts_at_seconds <= other.starts_at_seconds
                        && event.starts_at_seconds + event.duration.unwrap_or(0)
                            > other.starts_at_seconds)
                        || (other.starts_at_seconds <= event.starts_at_seconds
                            && other.starts_at_seconds + other.duration.unwrap_or(0)
                                > event.starts_at_seconds)
                    {
                        overlap_total += 1;
                        if j < i {
                            overlap_index += 1;
                        }
                    }
                }
                day_events[i].overlap_total = overlap_total;
                day_events[i].overlap_index = overlap_index;
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
            context.insert(&format!("week_date_{}", i), &formatted);
        }
        context.insert("week_dates", &week_dates);
        context.insert(
            "day_names",
            &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
        );

        context.insert("selected_year", &pivot.year());
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

pub fn routes() -> Scope {
    web::scope("")
        .service(sources)
        .service(local)
        .service(source)
        .service(me)
        .service(list)
        .service(calendar)
        .service(admin)
}

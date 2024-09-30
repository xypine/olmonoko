use std::collections::HashMap;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use chrono::NaiveDateTime;
use chrono::NaiveTime;
use chrono_tz::Tz;
use icalendar::Calendar;
use icalendar::Component;
use icalendar::DatePerhapsTime;
use icalendar::Event as VEvent;
use icalendar::EventLike;
use regex::Regex;
use rrule::RRuleSet;
use sha2::Digest;
use sqlx::Executor;
use sqlx::Postgres;
use tracing::info_span;
use tracing::Instrument;

use olmonoko_common::models::event::remote::NewRemoteEvent;
use olmonoko_common::models::event::remote::NewRemoteEventOccurrence;
use olmonoko_common::models::event::Priority;
use olmonoko_common::models::event::DEFAULT_PRIORITY;
use olmonoko_common::models::ics_source::IcsSource;
use olmonoko_common::models::ics_source::RawIcsSource;
use olmonoko_common::utils::time::timestamp;

use crate::db_utils::ical::EnhancedIcalendarEvent;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ProcessedData {
    pub events: Vec<NewRemoteEvent>,
    pub event_occurrences: Vec<Vec<NewRemoteEventOccurrence>>,
    pub skipped_event_ids: Vec<String>,
}

pub(crate) fn process_events(source: &IcsSource, events: Vec<VEvent>, tz: Tz) -> ProcessedData {
    let flatten_ts_with_tz = |dt| flatten_ts(dt, tz);
    let mut skipped = vec![];

    let (active_events, occurrences): (Vec<_>, Vec<_>) = events
        .into_iter()
        .flat_map(|event| {
            let uid = event.get_uid().map(|s| s.to_string());
            if uid.is_none() {
                return vec![];
            }
            let uid = uid.unwrap();

            let rrule = event.property_value("RRULE").map(|v| v.to_string());
            let dt_stamp = event.get_timestamp().map(|dt| dt.timestamp());
            let dt_start = event.get_start().and_then(flatten_ts_with_tz);
            let dt_end = event
                .get_end_auto()
                .map(|dt| dt.with_timezone(&tz).timestamp());
            let duration = match (dt_start, dt_end) {
                (Some(dt_start), Some(dt_end)) => Some(dt_end - dt_start),
                (_, _) => None,
            };
            let all_day = event
                .get_start()
                .map(|dt| matches!(dt, DatePerhapsTime::Date(_)))
                .unwrap_or(false);
            let summary = event.get_summary().map(|s| s.to_string());
            let location = event.get_location().map(|s| s.to_string());
            let description = event.get_description().map(|s| s.to_string());

            let occurrences: Vec<NewRemoteEventOccurrence> = get_event_occurrences(event, dt_start);

            let mut event = NewRemoteEvent {
                event_source_id: source.id,
                priority_override: None,
                rrule,
                uid: uid.clone(),
                dt_stamp,
                all_day,
                duration: duration.map(|d| d as i32),
                summary,
                location,
                description,
                tags: vec![],
            };
            let occurrences_start = occurrences.iter().map(|o| o.starts_at).collect();
            if let Some(template) = &source.import_template {
                match render_import_template(template, &event, occurrences_start) {
                    Ok(new_event) => {
                        if let Some(new_event) = new_event {
                            event = new_event;
                        } else {
                            // template requested to skip this event
                            skipped.push(uid);
                            return vec![];
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            source_id = source.id,
                            "Failed to render import template: {}",
                            e
                        );
                    }
                }
            }

            vec![(event, occurrences)]
        })
        .unzip();
    ProcessedData {
        events: active_events,
        event_occurrences: occurrences,
        skipped_event_ids: skipped,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Failed to fetch source: {0}")]
    FetchError(#[from] FetchError),
    #[error("Source not found")]
    SourceNotFound,
    #[error("Failed to insert events: {0}")]
    InsertEventsError(#[from] sqlx::Error),
}
pub(crate) async fn sync_source<C>(conn: &mut C, source_id: i32) -> Result<bool, SyncError>
where
    for<'e> &'e mut C: Executor<'e, Database = Postgres>,
{
    let source = sqlx::query_as!(
        RawIcsSource,
        "SELECT * FROM ics_sources WHERE id = $1",
        source_id
    )
    .fetch_one(&mut *conn)
    .await
    .map(|raw| (raw, None))
    .map(IcsSource::from)
    .map_err(|_| SyncError::SourceNotFound)?;

    // Fetch new events
    let fetched_at = timestamp();
    let (events, tz, new_hash) = if let FetchResult::Updated(data) =
        fetch_source(&source.url, source.file_hash.clone()).await?
    {
        data
    } else {
        tracing::info!("No new events (file hash match)");
        return Ok(false);
    };

    let processed = process_events(&source, events, tz);
    let mut hasher = DefaultHasher::new();
    processed.hash(&mut hasher);
    let object_hash = hasher.finish().to_string();
    let current_object_hash_version = crate::get_version();
    if source.object_hash.is_some_and(|hash| hash == object_hash)
        && source
            .object_hash_version
            .is_some_and(|version| version == current_object_hash_version)
    {
        tracing::info!("No new events (object hash match)");
        sqlx::query!(
            "UPDATE ics_sources SET last_fetched_at = $1, file_hash = $2 WHERE id = $3",
            fetched_at,
            new_hash,
            source_id
        )
        .execute(&mut *conn)
        .await?;
        return Ok(false);
    }

    let active_events = processed.events;

    if !source.persist_events {
        let future_event_ids: Vec<_> = active_events
            .iter()
            .map(|event| event.uid.clone())
            .collect();
        // Remove existing events for this source
        sqlx::query!(
            "DELETE FROM events WHERE event_source_id = $1 AND NOT ( uid = ANY($2) )",
            source_id,
            &future_event_ids
        )
        .execute(&mut *conn)
        .await?;
    }

    let mut event_occurrences = processed.event_occurrences;
    let skipped_events = processed.skipped_event_ids;

    // Insert new events
    let events_len = active_events.len();
    tracing::info!("Inserting {} events", events_len);
    tracing::info!("Skipped {} events", skipped_events.len());
    assert_eq!(events_len, event_occurrences.len());
    for skipped in skipped_events {
        // Remove skipped events
        sqlx::query!(
            "DELETE FROM events WHERE event_source_id = $1 AND uid = $2",
            source_id,
            skipped
        )
        .execute(&mut *conn)
        .await?;
    }
    let mut idmap = vec![];
    for event in active_events {
        let all_day = if source.all_as_allday {
            true
        } else {
            event.all_day
        };
        let inserted_id = sqlx::query_scalar!(r#"
            INSERT INTO events (event_source_id, uid, dt_stamp, all_day, duration, summary, location, description, rrule, priority_override)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT(event_source_id, uid, coalesce(rrule, '')) DO UPDATE SET
                dt_stamp = excluded.dt_stamp,
                priority_override = excluded.priority_override,
                all_day = excluded.all_day,
                duration = excluded.duration,
                summary = excluded.summary,
                location = excluded.location,
                description = excluded.description
            RETURNING id;
            "#, event.event_source_id, event.uid, event.dt_stamp, all_day, event.duration, event.summary, event.location, event.description, event.rrule, event.priority_override)
            .fetch_one(&mut *conn)
            .await?;
        idmap.push(inserted_id);

        // update tags
        sqlx::query!(
            "DELETE FROM event_tags WHERE remote_event_id = $1",
            inserted_id
        )
        .execute(&mut *conn)
        .await?;
        for tag in &event.tags {
            sqlx::query!(
                "INSERT INTO event_tags (remote_event_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                inserted_id,
                tag
            )
            .execute(&mut *conn)
            .await?;
        }
    }
    for i in 0..events_len {
        let inserted_id = idmap[i];
        for occurrence in &mut event_occurrences[i] {
            occurrence.event_id = inserted_id;
        }
    }
    let event_occurrences: Vec<_> = event_occurrences.into_iter().flatten().collect();
    for even_occurrence in event_occurrences {
        sqlx::query!(
            r#"
                INSERT INTO event_occurrences (event_id, starts_at, from_rrule)
                VALUES ($1, $2, $3)
                ON CONFLICT(event_id, starts_at) DO NOTHING;
            "#,
            even_occurrence.event_id,
            even_occurrence.starts_at,
            even_occurrence.from_rrule
        )
        .execute(&mut *conn)
        .await
        .unwrap();
    }

    // update source last fetched and file hash
    sqlx::query!(
        "UPDATE ics_sources SET last_fetched_at = $1, file_hash = $2, object_hash = $3, object_hash_version = $4 WHERE id = $5",
        fetched_at,
        new_hash,
        object_hash,
        current_object_hash_version,
        source_id
    )
    .execute(&mut *conn)
    .await?;

    Ok(true)
}

pub async fn sync_all() -> Result<(), anyhow::Error> {
    let conn = crate::get_conn().await?;
    let sources: Vec<IcsSource> = sqlx::query_as!(RawIcsSource, "SELECT * FROM ics_sources")
        .fetch_all(&conn)
        .await?
        .into_iter()
        .map(|raw| (raw, None))
        .map(IcsSource::from)
        .collect();

    let mut tasks = vec![];
    for source in sources {
        let source_id = source.id;
        let source_name = source.name;
        let conn = conn.clone();
        tasks.push(tokio::spawn(async move {
            let mut tx = conn.begin().await?;
            if let Err(e) = sync_source(&mut *tx, source_id)
                .instrument(info_span!("sync_source", source_id, source_name))
                .await
            {
                tracing::error!(source_id, source_name, "Failed to sync source: {e:?}");
                tx.rollback().await?;
            } else {
                tx.commit().await?;
            }
            Ok::<_, anyhow::Error>(())
        }));
    }
    for task in tasks {
        let res = task.await?;
        if let Err(e) = res {
            // do nothing
            tracing::error!("Failed to sync source: {}", e);
        }
    }

    Ok(())
}

fn flatten_ts(dt: DatePerhapsTime, tz: Tz) -> Option<i64> {
    Some(match dt {
        DatePerhapsTime::Date(date) => {
            let dt = match NaiveDateTime::new(date, NaiveTime::MIN).and_local_timezone(tz) {
                chrono::offset::LocalResult::Single(dt) => dt,
                chrono::offset::LocalResult::Ambiguous(earliest, _latest) => earliest,
                chrono::offset::LocalResult::None => return None,
            };
            dt.timestamp()
        }
        DatePerhapsTime::DateTime(datetime) => {
            datetime.try_into_utc()?.with_timezone(&tz).timestamp()
        }
    })
}

fn get_event_occurrences(event: VEvent, start: Option<i64>) -> Vec<NewRemoteEventOccurrence> {
    const MAX_OCCURRENCES: u16 = 10_000;
    // +- 10 years
    let max_delta = chrono::Duration::days(365 * 10);
    let rrule_min = (chrono::offset::Utc::now() - max_delta).with_timezone(&rrule::Tz::UTC);
    let rrule_max = (chrono::offset::Utc::now() + max_delta).with_timezone(&rrule::Tz::UTC);
    let mut events: Vec<NewRemoteEventOccurrence> = vec![];
    if let Some(dt_start) = event.properties().get("DTSTART") {
        if let Some(start) = start {
            events.push(NewRemoteEventOccurrence {
                event_id: -1, // placeholder, shouldn't exist
                starts_at: start,
                from_rrule: false,
            });
            if let Some(rrule_str) = event.property_value("RRULE") {
                // parse
                let dt_start_str = format!("DTSTART:{}", dt_start.value());
                let parse_result: Result<RRuleSet, _> =
                    format!("{dt_start_str}\n{rrule_str}").parse();
                match parse_result {
                    Ok(rrule) => {
                        // TODO: Revise limit to be time-based or some clever shit
                        // alternatively we could also just pass the RRULE to the client but that
                        // might make automations harder in the future?
                        let rrule = rrule.after(rrule_min).before(rrule_max);
                        let rrule_result = rrule.all(MAX_OCCURRENCES);
                        tracing::trace!("Rrule will add {} events", rrule_result.dates.len(),);
                        for date in rrule_result.dates {
                            let ts = date.timestamp();
                            if ts == start {
                                continue; // no need to have duplicate events
                            }
                            events.push(NewRemoteEventOccurrence {
                                event_id: -1,
                                starts_at: ts,
                                from_rrule: true,
                            });
                        }
                    }
                    Err(error) => {
                        tracing::warn!(rrule_str, "Failed to parse rrule: {}", error);
                    }
                }
            }
        } else {
            tracing::warn!("DTSTART was defined but start wasn't: {:#?}", event);
        }
    }
    events
}

fn parse_events(ics: String) -> Result<(Vec<VEvent>, Tz), String> {
    tracing::debug!("Parsing source");
    let calendar = ics.parse::<Calendar>()?;
    let events = calendar
        .components
        .iter()
        .filter_map(|event| match event {
            icalendar::CalendarComponent::Event(event) => Some(event.clone()),
            _ => None,
        })
        .collect();
    let tz = calendar.get_timezone();
    tracing::debug!("Got timezone: {:?}", tz);
    let tz = match tz {
        Some(tz) => tz.parse().unwrap_or(Tz::UTC),
        None => Tz::UTC,
    };
    tracing::debug!("Parsed timezone: {:?}", tz);
    Ok((events, tz))
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("Failed to fetch source: {0}")]
    FetchError(#[from] reqwest::Error),
    #[error("Failed to parse source: {0}")]
    ParseError(String),
}
#[derive(Debug, Clone, PartialEq)]
pub enum FetchResult {
    Unchanged,
    Updated((Vec<VEvent>, Tz, String)),
}
async fn fetch_source(url: &str, previous_hash: Option<String>) -> Result<FetchResult, FetchError> {
    tracing::info!("Fetching source: {}", url);
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    let new_hash = format!("{:x}", sha2::Sha256::digest(body.as_bytes()));
    if previous_hash.is_some_and(|phash| new_hash.as_str() == phash) {
        return Ok(FetchResult::Unchanged);
    }
    let parsed = parse_events(body).map_err(FetchError::ParseError)?;
    Ok(FetchResult::Updated((parsed.0, parsed.1, new_hash)))
}

#[derive(Debug, thiserror::Error)]
pub enum ImportTemplateError {
    #[error("Failed to render template: {0:#?}")]
    RenderError(#[from] tera::Error),
    #[error("Failed to parse rendered template \"{1}\": {0:#?}")]
    ParseError(serde_json::Error, String),
}
pub fn render_import_template(
    template: &str,
    event: &NewRemoteEvent,
    occurrences: Vec<i64>,
) -> Result<Option<NewRemoteEvent>, ImportTemplateError> {
    let mut context = tera::Context::new();
    context.insert("default_priority", &DEFAULT_PRIORITY);

    context.insert("priority_override", &event.priority_override);

    context.insert("rrule", &event.rrule);
    context.insert("dt_stamp", &event.dt_stamp);
    context.insert("all_day", &event.all_day);
    context.insert("duration", &event.duration);

    context.insert("summary", &event.summary);
    context.insert(
        "description",
        &event.description.as_ref().map(|s| s.replace('\n', "\\n")),
    );
    context.insert("location", &event.location);
    context.insert("uid", &event.uid);

    let now = timestamp();
    context.insert("now", &now);

    let next_occurrence = occurrences.iter().min();
    context.insert("next_occurrence", &next_occurrence);

    let mut past = vec![];
    let mut future = vec![];
    let mut ongoing = vec![];
    for occurrence in occurrences {
        if occurrence < now {
            if occurrence + event.duration.unwrap_or(0) as i64 > now {
                ongoing.push(occurrence);
            } else {
                past.push(occurrence);
            }
        } else {
            future.push(occurrence);
        }
    }
    context.insert("past_occurrences", &past);
    context.insert("future_occurrences", &future);

    let mut tera = tera::Tera::default();
    // disallow env access
    // see https://github.com/Keats/tera/issues/677
    tera.register_function("get_env", |_: &_| Ok(serde_json::json!("")));
    // handy for removing timestamps from descriptions
    fn replace_rg(
        v: &tera::Value,
        map: &HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        if let tera::Value::String(s) = v {
            if let Some(tera::Value::String(regex)) = map.get("regex") {
                if let Some(tera::Value::String(target)) = map.get("target") {
                    match Regex::new(regex) {
                        Ok(re) => {
                            let result = re.replace_all(s, target).to_string();
                            return tera::Result::Ok(tera::Value::String(result));
                        }
                        Err(_) => {
                            return tera::Result::Err(tera::Error::msg("invalid regex"));
                        }
                    }
                }
                return tera::Result::Err(tera::Error::msg("missing target"));
            }
            return tera::Result::Err(tera::Error::msg("missing regex"));
        }
        tera::Result::Err(tera::Error::msg("replace_rg: value must be a string"))
    }
    tera.register_filter("replace_rg", replace_rg);

    let result = tera.render_str(template, &context)?;
    let parsed: ImportTemplateDelta =
        serde_json::from_str(&result).map_err(|e| ImportTemplateError::ParseError(e, result))?;
    if parsed.skip {
        return Ok(None);
    }
    let mut new_event = event.clone();
    // apply delta
    new_event.priority_override = parsed.priority_override.or(event.priority_override);
    new_event.all_day = parsed.all_day.unwrap_or(event.all_day);
    new_event.duration = parsed.duration.or(event.duration);
    new_event.summary = parsed.summary.or(event.summary.clone());
    new_event.description = parsed.description.or(event.description.clone());
    new_event.location = parsed.location.or(event.location.clone());
    new_event.tags = parsed.tags;

    Ok(Some(new_event))
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportTemplateDelta {
    #[serde(default)]
    pub skip: bool,
    #[serde(default)]
    pub priority_override: Option<Priority>,
    #[serde(default)]
    pub all_day: Option<bool>,
    #[serde(default)]
    pub duration: Option<i32>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub fn test_import_template(template: &str) -> Result<(), ImportTemplateError> {
    let test_rrule = "FREQ=DAILY".to_string();
    let test_summary = "OLMONOKO::TEST".to_string();
    let event = olmonoko_common::models::event::remote::NewRemoteEvent {
        event_source_id: 1,
        priority_override: Some(1),
        rrule: Some(test_rrule),
        dt_stamp: Some(1234567890),
        all_day: true,
        duration: Some(3600),
        summary: Some(test_summary),
        description: Some("Test".to_string()),
        location: Some("Test".to_string()),
        uid: "test".to_string(),
        tags: vec![],
    };
    let _result =
        crate::logic::source_processing::render_import_template(template, &event, vec![])?;
    //    .context("Template didn't return anything")?;
    //assert_eq!(
    //    result,
    //    crate::models::event::remote::NewRemoteEvent {
    //        event_source_id: 1,
    //        priority_override: Some(1),
    //        rrule: Some("FREQ=DAILY".to_string()),
    //        dt_stamp: Some(1234567890),
    //        all_day: true,
    //        duration: Some(3600),
    //        summary: Some("Test".to_string()),
    //        description: Some("Test".to_string()),
    //        location: Some("Test".to_string()),
    //        uid: "test".to_string(),
    //        tags: vec![],
    //    }
    //);

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn render_empty_import_template() {
        let template = r#"
{}
"#;
        let event = olmonoko_common::models::event::remote::NewRemoteEvent {
            event_source_id: 1,
            priority_override: Some(1),
            rrule: Some("FREQ=DAILY".to_string()),
            dt_stamp: Some(1234567890),
            all_day: true,
            duration: Some(3600),
            summary: Some("Test".to_string()),
            description: Some("Test".to_string()),
            location: Some("Test".to_string()),
            uid: "test".to_string(),
            tags: vec![],
        };
        let result =
            crate::logic::source_processing::render_import_template(template, &event, vec![])
                .unwrap()
                .unwrap();
        assert_eq!(
            result,
            olmonoko_common::models::event::remote::NewRemoteEvent {
                event_source_id: 1,
                priority_override: Some(1),
                rrule: Some("FREQ=DAILY".to_string()),
                dt_stamp: Some(1234567890),
                all_day: true,
                duration: Some(3600),
                summary: Some("Test".to_string()),
                description: Some("Test".to_string()),
                location: Some("Test".to_string()),
                uid: "test".to_string(),
                tags: vec![],
            }
        );
    }

    #[test]
    fn render_import_template() {
        let template = r#"
        {
            "priority_override": 2,
            "all_day": false,
            "duration": 7200,
            "summary": "Test2",
            "description": "Test2",
            "location": "Test2",
            "tags": ["tag1"]
        }
        "#;
        let event = olmonoko_common::models::event::remote::NewRemoteEvent {
            event_source_id: 1,
            priority_override: Some(1),
            rrule: Some("FREQ=DAILY".to_string()),
            dt_stamp: Some(1234567890),
            all_day: true,
            duration: Some(3600),
            summary: Some("Test".to_string()),
            description: Some("Test".to_string()),
            location: Some("Test".to_string()),
            uid: "test".to_string(),
            tags: vec![],
        };
        let result =
            crate::logic::source_processing::render_import_template(template, &event, vec![])
                .unwrap()
                .unwrap();
        assert_eq!(
            result,
            olmonoko_common::models::event::remote::NewRemoteEvent {
                event_source_id: 1,
                priority_override: Some(2),
                rrule: Some("FREQ=DAILY".to_string()),
                dt_stamp: Some(1234567890),
                all_day: false,
                duration: Some(7200),
                summary: Some("Test2".to_string()),
                description: Some("Test2".to_string()),
                location: Some("Test2".to_string()),
                uid: "test".to_string(),
                tags: vec!["tag1".to_string()],
            }
        );
    }
}

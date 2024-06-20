use actix_web::web;
use itertools::Itertools;

use crate::{
    models::{
        bills::RawBill,
        event::{
            local::{LocalEvent, RawLocalEvent},
            remote::{RawRemoteEvent, RemoteEvent},
            Event, EventOccurrence, DEFAULT_PRIORITY,
        },
    },
    routes::AppState,
};

use super::event_filters::EventFilter;

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
            AND ($2 IS NULL OR event.starts_at + event.duration > $2)
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

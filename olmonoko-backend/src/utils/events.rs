use actix_web::web;
use itertools::Itertools;

use crate::{
    models::{
        attendance::{Attendance, RawAttendance},
        bills::RawBill,
        event::{
            local::{LocalEvent, RawLocalEvent}, remote::{RawRemoteEvent, RemoteEvent}, Event, EventOccurrence, Priority, DEFAULT_PRIORITY
        }, user::UserId,
    },
    routes::AppState,
};

use super::event_filters::EventFilter;

pub async fn get_user_local_events(
    data: &web::Data<AppState>,
    user_id: UserId,
    autodescription: bool,
    filter: &EventFilter,
) -> Vec<LocalEvent> {
    let min_priority = parse_priority(filter.min_priority);
    let max_priority = parse_priority(filter.max_priority);
    // let tags = filter.tags.clone().map(|tags| tags.join(","));
    // let exclude_tags = filter.exclude_tags.clone().map(|tags| tags.join(","));
    sqlx::query!(
        r#"
        SELECT event.*, 
            bill.id as "bill_id?", 
            bill.payee_account_number as "payee_account_number?", 
            bill.reference as "reference?", 
            bill.amount as "amount?",
            bill.created_at as "bill_created_at?", 
            bill.updated_at as "bill_updated_at?",
            bill.payee_name as "payee_name?",
            bill.payee_email as "payee_email?",
            bill.payee_address as "payee_address?",
            bill.payee_phone as "payee_phone?",
            -- STRING_AGG(tag.tag, ',') AS tags,
            attendance.planned as "planned?",
            attendance.planned_starts_at as "planned_starts_at?",
            attendance.planned_duration as "planned_duration?",
            attendance.actual as "actual?",
            attendance.actual_starts_at as "actual_starts_at?",
            attendance.actual_duration as "actual_duration?",
            attendance.created_at as "attendance_created_at?",
            attendance.updated_at as "attendance_updated_at?"
        FROM local_events AS event
        LEFT JOIN bills AS bill 
            ON bill.local_event_id = event.id 
        LEFT JOIN attendance
            ON attendance.local_event_id = event.id
        LEFT JOIN event_tags AS tag 
            ON tag.local_event_id = event.id
        WHERE event.user_id = $1 
            AND ($2::bigint IS NULL OR event.starts_at + COALESCE(event.duration, 0) > $2)
            AND ($3::bigint IS NULL OR event.starts_at < $3) 
            AND (COALESCE(NULLIF(event.priority, 0), $6) >= $4 OR $4 IS NULL)
            AND (COALESCE(NULLIF(event.priority, 0), $6) <= $5 OR $5 IS NULL)
            AND ($7::text IS NULL OR event.summary LIKE $7)
            AND ($8::text[] IS NULL OR tag.tag = ANY($8))
            AND ($9::text[] IS NULL OR tag IS NULL OR (
                SELECT tag.tag
                FROM event_tags AS tag
                WHERE tag.local_event_id = event.id
                AND tag.tag = ANY($9)
            ) IS NULL)
        -- GROUP BY event.id
        ORDER BY event.starts_at;
        "#,
        user_id,
        filter.after,
        filter.before,
        min_priority,
        max_priority,
        DEFAULT_PRIORITY,
        filter.summary_like,
        filter.tags.as_deref(),
        filter.exclude_tags.as_deref(),
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
                .payee_account_number.unwrap()
                ,
            reference: event.reference.unwrap(),
            amount: event.amount.unwrap(),
            created_at: event.bill_created_at.unwrap(),
            updated_at: event.bill_updated_at.unwrap(),
            payee_name: event.payee_name,
            payee_email: event.payee_email,
            payee_address: event.payee_address,
            payee_phone: event.payee_phone,
        });
        let attendance = event
            .attendance_created_at
            .map(|created_at| RawAttendance {
                created_at,
                updated_at: event
                    .attendance_updated_at.unwrap(),
                planned: event.planned.unwrap(),
                planned_starts_at: event.planned_starts_at,
                planned_duration: event.planned_duration,
                actual: event.actual.unwrap(),
                actual_starts_at: event.actual_starts_at,
                actual_duration: event.actual_duration,
                user_id: event.user_id,
                local_event_id: Some(event.id),
                remote_event_id: None,
            })
            .map(|a| {
                Attendance::from((a, event.starts_at, event.duration))
            });
        // let tags = event.tags.unwrap_or_default();
        let tags = "".to_string();
        LocalEvent::from((
            raw_event,
            raw_bill,
            autodescription,
            tags.as_str(),
            attendance,
        ))
    })
    .collect()
}

pub fn parse_priority(priority: Option<i32>) -> Option<Priority> {
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
    user_id: Option<UserId>,
    filter: &EventFilter,
) -> Vec<(RemoteEvent, i64, bool)> {
    let min_priority = parse_priority(filter.min_priority);
    let max_priority = parse_priority(filter.max_priority);
    // let tags = filter.tags.clone().map(|tags| tags.join(","));
    // let exclude_tags = filter.exclude_tags.clone().map(|tags| tags.join(","));
    sqlx::query!(
        r#"
        SELECT 
            e.*, 
            p.priority, 
            o.starts_at, 
            o.from_rrule,
            attendance.planned as "planned?",
            attendance.planned_starts_at as "planned_starts_at?",
            attendance.planned_duration as "planned_duration?",
            attendance.actual as "actual?",
            attendance.actual_starts_at as "actual_starts_at?",
            attendance.actual_duration as "actual_duration?",
            attendance.created_at as "attendance_created_at?",
            attendance.updated_at as "attendance_updated_at?"
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
            AND ($4::integer IS NULL OR (p.priority IS NOT NULL AND COALESCE(NULLIF(e.priority_override, 0), $6) >= $4) OR COALESCE(NULLIF(p.priority, 0), $6) >= $4)
            -- max_priority is null or (source_in_calendar and event_priority_override <= max_priority) and source_priority <= max_priority
            AND ($5::integer IS NULL OR (p.priority IS NOT NULL AND COALESCE(NULLIF(e.priority_override, 0), $6) <= $5) AND COALESCE(NULLIF(p.priority, 0), $6) <= $5)
        LEFT JOIN event_tags AS tag
            ON tag.remote_event_id = e.id
        LEFT JOIN attendance
            ON attendance.remote_event_id = e.id
        WHERE 
            ($2::bigint IS NULL OR o.starts_at + COALESCE(e.duration, 0) > $2::bigint) 
            AND ($3::bigint IS NULL OR o.starts_at < $3) 
            AND ($7::text IS NULL OR e.summary LIKE $7)
            AND ($8::text[] IS NULL OR tag.tag = ANY($8))
            AND ($9::text[] IS NULL OR tag IS NULL OR (
                SELECT tag.tag
                FROM event_tags AS tag
                WHERE tag.remote_event_id = e.id
                AND tag.tag = ANY($9)
            ) IS NULL)
        ORDER BY 
            o.starts_at;
        "#,
        user_id,
        filter.after,
        filter.before,
        min_priority,
        max_priority,
        DEFAULT_PRIORITY,
        filter.summary_like,
        filter.tags.as_deref(),
        filter.exclude_tags.as_deref(),
    )
    .fetch_all(&data.conn)
    .await
    .expect("Failed to get events")
    .into_iter()
    .map(|event| {
        let attendance = event
            .attendance_created_at
            .and_then(|created_at| user_id.map(|user_id| (created_at, user_id)))
            .map(|(created_at, user_id)| RawAttendance {
                created_at,
                updated_at: event
                    .attendance_updated_at.unwrap(),
                planned: event.planned.unwrap(),
                planned_starts_at: event.planned_starts_at,
                planned_duration: event.planned_duration,
                actual: event.actual.unwrap(),
                actual_starts_at: event.actual_starts_at,
                actual_duration: event.actual_duration,
                user_id,
                local_event_id: Some(event.id),
                remote_event_id: None,
            })
            .map(|a| {
                Attendance::from((a, event.starts_at, event.duration))
            });
        (
            RemoteEvent::from((RawRemoteEvent {
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
            }, attendance)), 
            event.starts_at,
            event.from_rrule,
        )
    })
    .collect()
}
pub async fn get_visible_events(
    data: &web::Data<AppState>,
    user_id: Option<UserId>,
    autodescription: bool,
    filter: &EventFilter,
) -> Vec<Event> {
    // remote
    let remote_events = get_visible_remote_events(data, user_id, filter).await;
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
        let local_events: Vec<Event> =
            get_user_local_events(data, user_id, autodescription, filter)
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
    user_id: Option<UserId>,
    autodescription: bool,
    filter: &EventFilter,
) -> Vec<EventOccurrence> {
    let events = get_visible_events(data, user_id, autodescription, filter).await;
    events
        .into_iter()
        .flat_map(Vec::<EventOccurrence>::from)
        .sorted_by_key(|event| event.starts_at.timestamp())
        .collect()
}

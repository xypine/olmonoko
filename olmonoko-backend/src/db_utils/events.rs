use actix_web::web;
use itertools::Itertools;

use olmonoko_common::{
    models::{
        bills::RawBill,
        event::{
            local::{LocalEvent, LocalEventId, RawLocalEvent},
            remote::{
                RawRemoteEvent, RawRemoteEventOccurrence, RemoteEvent, RemoteEventOccurrence,
                RemoteEventOccurrenceId,
            },
            Event, EventOccurrence, Priority, DEFAULT_PRIORITY,
        },
        user::UserId,
    },
    AppState,
};

use olmonoko_common::utils::event_filters::EventFilter;

pub async fn get_user_local_events(
    data: &web::Data<AppState>,
    user_id: UserId,
    autodescription: bool,
    filter: &EventFilter,
) -> Vec<LocalEvent> {
    let min_priority = parse_priority(filter.min_priority);
    let max_priority = parse_priority(filter.max_priority);
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
            STRING_AGG(tag.tag, ',') AS tags
        FROM local_events AS event
        LEFT JOIN bills AS bill 
            ON bill.local_event_id = event.id 
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
            AND ($10::boolean IS NULL OR event.attendance_planned = $10)
            AND ($11::boolean IS NULL OR event.attendance_actual = $11)
        GROUP BY event.id, bill.id
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
        filter.attendance_planned,
        filter.attendance_actual
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
            attendance_planned: event.attendance_planned,
            attendance_actual: event.attendance_actual,
            auto_imported: event.auto_imported,
        };
        let raw_bill = event.bill_id.map(|bill_id| RawBill {
            id: bill_id,
            local_event_id: event.id,
            payee_account_number: event.payee_account_number.unwrap(),
            reference: event.reference.unwrap(),
            amount: event.amount.unwrap(),
            created_at: event.bill_created_at.unwrap(),
            updated_at: event.bill_updated_at.unwrap(),
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

pub async fn get_visible_event_occurrence_with_event(
    data: &web::Data<AppState>,
    user_id: Option<UserId>,
    occurrence_id: RemoteEventOccurrenceId,
) -> Result<Option<(RemoteEvent, RemoteEventOccurrence)>, sqlx::Error> {
    let raw = sqlx::query!(
        r#"
        SELECT
            p.priority AS source_priority,
            o.id AS occurrence_id,
            o.starts_at AS occurrence_starts_at,
            o.from_rrule AS occurrence_from_rrule,
            e.* AS event
        FROM
            event_occurrences AS o
        INNER JOIN
            events AS e
            ON o.event_id = e.id 
        INNER JOIN
            ics_sources AS s
            ON e.event_source_id = s.id
            AND (s.user_id = $1 OR s.is_public)
        INNER JOIN
            ics_source_priorities AS p
            ON p.user_id = $1
            AND p.ics_source_id = s.id
        WHERE
            o.id = $2
        "#,
        user_id,
        occurrence_id
    )
    .fetch_optional(&data.conn)
    .await?;
    Ok(raw.map(|r| {
        let event = RemoteEvent::from((
            RawRemoteEvent {
                priority_override: r.priority_override,
                rrule: r.rrule,
                id: r.id,
                event_source_id: r.event_source_id,
                uid: r.uid,
                summary: r.summary,
                dt_stamp: r.dt_stamp,
                all_day: r.all_day,
                duration: r.duration,
                location: r.location,
                description: r.description,
            },
            r.source_priority,
        ));
        let occurrence = RemoteEventOccurrence::from(RawRemoteEventOccurrence {
            id: r.occurrence_id,
            event_id: r.id,
            from_rrule: r.occurrence_from_rrule,
            starts_at: r.occurrence_starts_at,
        });
        (event, occurrence)
    }))
}

async fn get_visible_remote_events(
    data: &web::Data<AppState>,
    user_id: Option<UserId>,
    filter: &EventFilter,
) -> Vec<(
    RemoteEvent,
    RemoteEventOccurrenceId,
    i64,
    Vec<LocalEventId>,
    bool,
)> {
    if filter.tags.is_some() {
        return vec![];
    }
    let min_priority = parse_priority(filter.min_priority);
    let max_priority = parse_priority(filter.max_priority);

    sqlx::query!(
        r#"
        SELECT
            e.*,
            p.priority,
            o.id AS occurrence_id,
            o.starts_at,
            array_agg(l.local_event_id) as "linked: Vec<Option<i32>>",
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
            AND ($4::integer IS NULL OR (p.priority IS NOT NULL AND COALESCE(NULLIF(e.priority_override, 0), $6) >= $4) OR COALESCE(NULLIF(p.priority, 0), $6) >= $4)
            -- max_priority is null or (source_in_calendar and event_priority_override <= max_priority) and source_priority <= max_priority
            AND ($5::integer IS NULL OR (p.priority IS NOT NULL AND COALESCE(NULLIF(e.priority_override, 0), $6) <= $5) AND COALESCE(NULLIF(p.priority, 0), $6) <= $5)
        LEFT JOIN
            remote_local_link AS l
            ON o.id = l.remote_occurrence_id
        WHERE
            ($2::bigint IS NULL OR o.starts_at + COALESCE(e.duration, 0) > $2::bigint)
            AND ($3::bigint IS NULL OR o.starts_at < $3)
            AND ($7::text IS NULL OR e.summary LIKE $7)
            --AND ($8::text[] IS NULL OR tag.tag = ANY($8))
            --AND ($9::text[] IS NULL OR tag IS NULL OR (
            --    SELECT tag.tag
            --    FROM event_tags AS tag
            --    WHERE tag.remote_event_id = e.id
            --    AND tag.tag = ANY($9)
            --) IS NULL)
        GROUP BY
            e.id, p.priority, o.starts_at, o.from_rrule, o.id
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
    )
    .fetch_all(&data.conn)
    .await
    .expect("Failed to get events")
    .into_iter()
    .map(|event| {
        let occurrence_linked_local: Vec<_> = event.linked.unwrap_or_default().into_iter().flatten().collect();
        (
            RemoteEvent::from((RawRemoteEvent {
                priority_override: event.priority_override,
                rrule: event.rrule,
                id: event.id,
                event_source_id: event.event_source_id,
                uid: event.uid,
                summary: event.summary,
                dt_stamp: event.dt_stamp,
                all_day: event.all_day,
                duration: event.duration,
                location: event.location,
                description: event.description,
            }, event.priority)),
            event.occurrence_id,
            event.starts_at,
            occurrence_linked_local,
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
    // NOTE: Add documentation, what does this do?
    // Does it just form Events from RemoteEvents?
    let mut events: Vec<Event> = remote_events
        .into_iter()
        .sorted_by_key(|(event, _, _, _, _)| event.id)
        .chunk_by(|(event, _, _, _, _)| event.id)
        .into_iter()
        .flat_map(|(_, group)| {
            let group: Vec<_> = group.collect();
            if group.is_empty() {
                None
            } else {
                let (event, _, _, _, _) = group.first().unwrap().clone();
                let meta = group
                    .into_iter()
                    .map(|(_, oid, starts_at, linked, _)| (oid, starts_at, linked))
                    .collect::<Vec<_>>();
                Some((event, meta))
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
    events.sort_by_key(|event| match event {
        Event::Local(event) => event.starts_at.timestamp(),
        Event::Remote(_, meta) => {
            meta.first()
                .expect("remote event missing any occurrence after aggregation")
                .1
        }
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

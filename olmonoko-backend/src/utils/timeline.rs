use std::collections::BTreeMap;

use actix_web::web;

use crate::{
    routes::AppState,
    utils::{event_filters::EventFilter, events::get_visible_event_occurrences},
};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Timeline {
    pub chunk_size: i64,
    pub by_date_normalized: Vec<(i64, f64)>,
    pub max: Option<usize>,
    pub min_date: Option<i64>,
    pub max_date: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum TimelineCompilationError {
    #[error("Database error: {0}")]
    DbErr(#[from] sqlx::Error),
}

pub async fn compile_timeline(
    data: &web::Data<AppState>,
    user_id: i64,
    filter: &EventFilter,
    chunk_size_seconds: i64,
) -> Result<Timeline, TimelineCompilationError> {
    tracing::debug!("Fetching timeline data...");
    // TODO: Ponder on the performance characteristics custom query vs easy generic implementation
    // let data = sqlx::query_scalar!(
    //     r#"
    //     SELECT
    //         starts_at
    //     FROM
    //         event_occurrences
    //     UNION ALL
    //     SELECT
    //         starts_at
    //     FROM
    //         local_events
    // "#
    // )
    // .fetch_all(&data.conn)
    // .await?;
    let event_occurrences = get_visible_event_occurrences(data, Some(user_id), false, filter).await;

    let data = event_occurrences
        .into_iter()
        .map(|occurrence| occurrence.starts_at.timestamp())
        .collect::<Vec<_>>();

    tracing::debug!("Grouping timeline data by date");
    let by_date = group_by_approx_date(data, chunk_size_seconds);
    Ok(if let Some(by_date) = by_date {
        tracing::info!("Max events / day (approx): {:?}", by_date.max);
        let normalized: Vec<_> = by_date
            .data
            .into_iter()
            .map(|(date, occurrences)| (date, ((occurrences as f64) / (by_date.max as f64))))
            .collect();
        Timeline {
            by_date_normalized: normalized,
            max: Some(by_date.max),
            min_date: Some(by_date.min_date),
            max_date: Some(by_date.max_date),
            chunk_size: chunk_size_seconds,
        }
    } else {
        tracing::info!("Max events / day: unknown");
        Timeline {
            by_date_normalized: vec![],
            max: None,
            min_date: None,
            max_date: None,
            chunk_size: chunk_size_seconds,
        }
    })
}

#[derive(Debug)]
struct ApproxByDate {
    data: Vec<(i64, usize)>,
    max: usize,
    min_date: i64,
    max_date: i64,
}

fn group_by_approx_date(starts_at: Vec<i64>, chunk_size: i64) -> Option<ApproxByDate> {
    let events_total = starts_at.len();
    if events_total == 0 {
        return None;
    }
    let mut max = 1;
    let mut min_date = i64::MAX;
    let mut max_date = i64::MIN;
    let mut aggregate: Vec<(i64, usize)> = vec![];
    let mut aggregate_len = 0;
    let mut position_map: BTreeMap<i64, usize> = BTreeMap::new();
    for ts in starts_at {
        let approx_date = ts / chunk_size;
        min_date = approx_date.min(min_date);
        max_date = approx_date.max(max_date);
        if let Some(pos) = position_map.get(&approx_date) {
            let total = &mut aggregate[*pos].1;
            *total += 1;
            max = max.max(*total)
        } else {
            aggregate.push((approx_date, 1));
            position_map.insert(approx_date, aggregate_len);
            aggregate_len += 1;
        }
    }

    tracing::info!("Grouped by date of {events_total} events resulted in {aggregate_len} chunks");

    Some(ApproxByDate {
        data: aggregate,
        max,
        min_date,
        max_date,
    })
}

#[cfg(test)]
mod tests {
    use super::group_by_approx_date;

    #[test]
    fn group_by_approx_sanity() {
        const ONE_DAY_S: i64 = 86_400;
        let one_week_seconds = 604_800;
        let starts_at = vec![1719181069, 1719181069 - one_week_seconds];
        let grouped_by_date = group_by_approx_date(starts_at, ONE_DAY_S).unwrap();

        assert_eq!(grouped_by_date.min_date, grouped_by_date.max_date - 7);

        assert_eq!(grouped_by_date.data.len(), 2);
        assert_eq!(grouped_by_date.max, 1);
        let (first_d, first_c) = grouped_by_date.data[0];
        let (second_d, second_c) = grouped_by_date.data[1];
        assert_ne!(first_d, second_d);
        assert_eq!(first_c, 1);
        assert_eq!(second_c, 1);
    }
}

use std::collections::BTreeMap;

use actix_web::web;

use crate::routes::AppState;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Timeline {
    relative_event_count_per_date: Vec<(i64, f64)>,
    max: Option<usize>,
    min_date: Option<i64>,
    max_date: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum TimelineCompilationError {
    #[error("Database error: {0}")]
    DbErr(#[from] sqlx::Error),
}

pub async fn compile_timeline(
    data: &web::Data<AppState>,
) -> Result<Timeline, TimelineCompilationError> {
    tracing::debug!("Fetching timeline data...");
    let data = sqlx::query_scalar!(
        r#"
        SELECT starts_at FROM event_occurrences
    "#
    )
    .fetch_all(&data.conn)
    .await?;

    tracing::debug!("Grouping timeline data by date");
    let by_date = group_by_approx_date(data);
    Ok(if let Some(by_date) = by_date {
        tracing::info!("Max events / day (approx): {:?}", by_date.max);
        let normalized: Vec<_> = by_date
            .data
            .into_iter()
            .map(|(date, occurrences)| (date, ((occurrences as f64) / (by_date.max as f64))))
            .collect();
        Timeline {
            relative_event_count_per_date: normalized,
            max: Some(by_date.max),
            min_date: Some(by_date.min_date),
            max_date: Some(by_date.max_date),
        }
    } else {
        tracing::info!("Max events / day: unknown");
        Timeline {
            relative_event_count_per_date: vec![],
            max: None,
            min_date: None,
            max_date: None,
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

fn group_by_approx_date(starts_at: Vec<i64>) -> Option<ApproxByDate> {
    if starts_at.is_empty() {
        return None;
    }
    const ONE_DAY_S: i64 = 86_400;
    let mut max = 1;
    let mut min_date = i64::MAX;
    let mut max_date = i64::MIN;
    let mut aggregate: Vec<(i64, usize)> = vec![];
    let mut aggregate_len = 0;
    let mut position_map: BTreeMap<i64, usize> = BTreeMap::new();
    for ts in starts_at {
        let approx_date = ts / ONE_DAY_S;
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
        let one_week_seconds = 604_800;
        let starts_at = vec![1719181069, 1719181069 - one_week_seconds];
        let grouped_by_date = group_by_approx_date(starts_at).unwrap();

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

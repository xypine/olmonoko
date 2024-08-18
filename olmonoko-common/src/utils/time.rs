use chrono::DateTime;
use chrono::Utc;

pub fn get_current_time() -> DateTime<Utc> {
    chrono::offset::Utc::now()
}

pub fn timestamp() -> i64 {
    get_current_time().timestamp()
}

pub fn from_timestamp(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp, 0).expect("Invalid timestamp")
}

pub fn from_form(dt_form: &str, tz_offset: i8) -> DateTime<Utc> {
    // FIX: This is stupid
    let dt = if dt_form.chars().filter(|c| *c == ':').count() == 2 {
        dt_form.to_string()
    } else {
        format!("{}:00", dt_form)
    };
    let tz = if tz_offset >= 0 {
        format!("+{:02}:00", tz_offset)
    } else {
        format!("-{:02}:00", -tz_offset)
    };
    let rfc = format!("{dt}{tz}");
    tracing::debug!("Parsing RFC3339 datetime: {}", rfc);
    let fixed =
        chrono::DateTime::parse_from_rfc3339(&rfc).expect("Failed to parse RFC3339 datetime");
    fixed.with_timezone(&Utc)
}

pub fn to_form(dt: DateTime<Utc>) -> Option<String> {
    dt.to_rfc3339()
        .split('+')
        .collect::<Vec<_>>()
        .first()
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use chrono::Timelike;

    use super::*;

    #[test]
    fn sanity() {
        let ts = timestamp();
        let dt = from_timestamp(ts);
        let now = get_current_time();
        // timestamps are measured in seconds, strip out nanoseconds
        let now = now.with_nanosecond(0).unwrap();
        assert_eq!(dt, now);
    }
}

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

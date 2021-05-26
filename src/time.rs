use chrono::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn create_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn format_timestamp(
    timestamp: u64,
) -> chrono::format::DelayedFormat<chrono::format::StrftimeItems<'static>> {
    let naive = NaiveDateTime::from_timestamp((timestamp / 1000) as i64, 0);
    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
    let newdate = datetime.format("%Y-%m-%d %H:%M:%S");
    return newdate;
}

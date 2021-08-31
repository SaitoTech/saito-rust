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
    datetime.format("%Y-%m-%d %H:%M:%S")
}

/// A helper for tracing. Get the amount of time passed. Only for use in a single task/thread/future.
pub struct TracingTimer {
    prev_timestamp: u64,
}
impl TracingTimer {
    /// Create a new TracingTimer
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> TracingTimer {
        TracingTimer {
            prev_timestamp: create_timestamp(),
        }
    }
    /// Gets the time passed since this method was called
    pub fn time_since_last(&mut self) -> u64 {
        let now = create_timestamp();
        let ret_val = now - self.prev_timestamp;
        self.prev_timestamp = create_timestamp();
        ret_val
    }
}
pub struct TracingAccumulator {
    prev_timestamp: u64,
    total: u64,
}

/// A helper for tracing. Get the amount of time for a series of. Only for use in a single task/thread/future.
impl TracingAccumulator {
    /// Create a new TracingAccumulator
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> TracingAccumulator {
        TracingAccumulator {
            prev_timestamp: create_timestamp(),
            total: 0,
        }
    }
    pub fn set_start(&mut self) {
        self.prev_timestamp = create_timestamp();
    }
    /// Accumulate the time passed since this method was called
    pub fn accumulate_time_since_start(&mut self) {
        // We throw out the first datapoint because we might be doing a lot of heavy lifting before we call this the first time...
        let now = create_timestamp();
        self.total += now - self.prev_timestamp;
        self.prev_timestamp = create_timestamp();
    }
    /// Get the total time accumulated
    pub fn finish(&self) -> u64 {
        // We could try to compenstate for the missing first datapoint by adding the average of the others, but... do we really care?
        self.total
    }
}

pub trait Persistable {
    fn save(&self);
    fn load(filename: &str) -> Self;
}

pub trait TimestampGenerator {
    fn get_timestamp(&mut self) -> u64 {
        create_timestamp()
    }
}

pub struct SystemTimestampGenerator {}
impl TimestampGenerator for SystemTimestampGenerator {
    // fn get_timestamp(&mut self) -> u64 {
    //     create_timestamp()
    // }
}

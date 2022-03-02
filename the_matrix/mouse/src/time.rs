pub use chrono::*;

pub trait Timestamp {
    fn timestamp_ns(&self) -> u64;
    fn timestamp_s(&self) -> u32;
}

impl<Tz: chrono::TimeZone> Timestamp for chrono::DateTime<Tz> {
    fn timestamp_ns(&self) -> u64 {
        self.timestamp() as u64 * 1_000_000_000 + self.timestamp_subsec_nanos() as u64
    }

    fn timestamp_s(&self) -> u32 {
        self.timestamp() as u32
    }
}

pub trait IntoDateTime {
    fn into_date_time(&self) -> DateTime<Utc>;
}

impl IntoDateTime for u32 {
    fn into_date_time(&self) -> DateTime<Utc> {
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(*self as i64, 0), Utc)
    }
}

impl IntoDateTime for u64 {
    fn into_date_time(&self) -> DateTime<Utc> {
        let timestamp = (*self / 1_000_000_000) as i64;
        let nsecs = (*self % 1_000_000_000) as u32;
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, nsecs), Utc)
    }
}

impl IntoDateTime for i64 {
    fn into_date_time(&self) -> DateTime<Utc> {
        // i64 * 1_000_000_000
        DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp(
                (*self / 1_000_000_000) as i64,
                (*self % 1_000_000_000).abs() as u32,
            ),
            Utc,
        )
    }
}

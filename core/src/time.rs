use chrono::{TimeZone, Utc};
use sea_orm::entity::prelude::DateTimeWithTimeZone;

pub fn ts_to_dt(ts: i64) -> DateTimeWithTimeZone {
    Utc.timestamp_opt(ts, 0)
        .single()
        .unwrap_or(chrono::DateTime::<Utc>::UNIX_EPOCH)
        .fixed_offset()
}

pub fn dt_to_ts(dt: DateTimeWithTimeZone) -> i64 {
    dt.timestamp()
}

#[cfg(test)]
pub fn opt_dt_to_ts(dt: Option<DateTimeWithTimeZone>) -> Option<i64> {
    dt.map(dt_to_ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_timestamp_conversions() {
        let sample_ts = 1_700_000_000i64;
        let dt = ts_to_dt(sample_ts);
        assert_eq!(dt_to_ts(dt), sample_ts);
        assert_eq!(opt_dt_to_ts(Some(dt)), Some(sample_ts));
        assert_eq!(opt_dt_to_ts(None), None);
    }

    #[test]
    fn handles_zero_unix_epoch() {
        let dt = ts_to_dt(0);
        assert_eq!(dt_to_ts(dt), 0);
    }
}

use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

/// Polymarket aligns 15m/5m markets to Eastern Time (ET).
pub fn period_start_et_unix(minutes: i64) -> i64 {
    let utc_now = chrono::Utc::now();
    period_start_et_unix_at(utc_now.timestamp(), minutes)
}

/// ET-aligned period start (Unix) that contains the given timestamp.
pub fn period_start_et_unix_at(ts_sec: i64, minutes: i64) -> i64 {
    let utc_dt = match chrono::Utc.timestamp_opt(ts_sec, 0).single() {
        Some(dt) => dt,
        None => return ts_sec,
    };
    let et = New_York;
    let et_dt = utc_dt.with_timezone(&et);
    let minute_floor = (et_dt.minute() as i64 / minutes) * minutes;
    let truncated_naive = et_dt
        .date_naive()
        .and_hms_opt(et_dt.hour(), minute_floor as u32, 0)
        .expect("valid ET period timestamp");
    let dt_et = et
        .from_local_datetime(&truncated_naive)
        .single()
        .or_else(|| et.from_local_datetime(&truncated_naive).earliest())
        .expect("ET period start");
    dt_et.timestamp()
}

pub fn current_15m_period_start() -> i64 {
    period_start_et_unix(15)
}

pub fn current_5m_period_start() -> i64 {
    period_start_et_unix(5)
}

pub fn is_last_5min_of_15m(now_ts: i64, period_15m_start: i64) -> bool {
    let elapsed = now_ts - period_15m_start;
    elapsed >= 10 * 60 && elapsed < 15 * 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_five_minute_window_bounds() {
        let start = 1_700_000_000;
        assert!(!is_last_5min_of_15m(start + 599, start));
        assert!(is_last_5min_of_15m(start + 600, start));
        assert!(is_last_5min_of_15m(start + 899, start));
        assert!(!is_last_5min_of_15m(start + 900, start));
    }

    #[test]
    fn rounds_timestamp_to_expected_period_start() {
        let ts = 1_700_001_234;
        let p15 = period_start_et_unix_at(ts, 15);
        let p5 = period_start_et_unix_at(ts, 5);
        assert!(ts >= p15 && ts < p15 + 900);
        assert!(ts >= p5 && ts < p5 + 300);
    }
}

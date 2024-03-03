use actix_web::cookie::time::Instant;
use chrono::prelude::DateTime;
use chrono::Datelike;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::Utc;
use dateparser::parse;

use std::time::{Duration, SystemTime};

pub const SEC_AS_MILI: u64 = 1000;
pub const MIN_AS_MILI: u64 = SEC_AS_MILI * 60; // 60000
pub const HOUR_AS_MILI: u64 = MIN_AS_MILI * 60; // 3600000
pub const DAY_AS_MILI: u64 = HOUR_AS_MILI * 24; // 86400000

/// Generates a current timestamp in milliseconds since the UNIX epoch.
///
/// # Returns
///
/// A `u64` representing the current timestamp in milliseconds.
pub fn generate_ts() -> u64 {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Error creating timestamp")
        .as_millis();
    let mut s_ts = format!("{}", now);
    while s_ts.len() < 13 {
        s_ts.push('0');
    }
    s_ts.parse::<u64>().unwrap()
}

/// Converts a UNIX timestamp in milliseconds to a `DateTime<Utc>`.
///
/// # Arguments
///
/// * `timestamp` - A UNIX timestamp in milliseconds.
///
/// # Returns
///
/// A `DateTime<Utc>` corresponding to the given timestamp.
pub fn timestamp_to_datetime(timestamp: u64) -> DateTime<Utc> {
    let mut s_ts = format!("{}", timestamp);
    s_ts.truncate(13);
    while s_ts.len() < 13 {
        s_ts.push('0');
    }
    let n_ts = s_ts.parse::<u64>().unwrap();
    let naive =
        NaiveDateTime::from_timestamp_opt(n_ts as i64 / 1000, (n_ts % 1000) as u32 * 1_000_000)
            .unwrap();
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
}

/// Converts a date string to a UNIX timestamp in milliseconds.
///
/// # Arguments
///
/// * `date_str` - The date string to convert.
///
/// # Returns
///
/// A `Result<u64, &'static str>` which is Ok containing the timestamp in milliseconds if successful, or an Err with an error message.
pub fn string_to_timestamp(date_str: &str) -> Result<u64, &'static str> {
    if let Ok(date) = parse(date_str) {
        let date = date.timestamp_millis();
        let mut s_ts = format!("{}", date);
        while s_ts.len() < 13 {
            s_ts.push('0');
        }

        if let Ok(ts) = s_ts.parse::<u64>() {
            return Ok(ts);
        } else {
            return Err("Unable to parse date string");
        }
    };

    Err("Unable to parse date string")
}

/// Converts a UNIX timestamp in milliseconds to a date string in ISO 8601 format.
///
/// # Arguments
///
/// * `ts` - A UNIX timestamp in milliseconds.
///
/// # Returns
///
/// A `String` representing the date in ISO 8601 format.
pub fn timestamp_to_string(ts: u64) -> String {
    let datetime = timestamp_to_datetime(ts);
    let timestamp_str = datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    timestamp_str
}

/// Converts a year, month, and day to a UNIX timestamp in milliseconds.
///
/// # Arguments
///
/// * `year` - The year component of the date.
/// * `month` - The month component of the date.
/// * `day` - The day component of the date.
///
/// # Returns
///
/// An `Option<u64>` which is Some containing the timestamp in milliseconds if the date is valid, or None if the date is invalid.
pub fn year_month_day_to_ts(year: u32, month: u32, day: u32) -> Option<u64> {
    let date = NaiveDate::from_ymd_opt(year as i32, month, day);

    match date {
        Some(date) => {
            if let Some(date) = date.and_hms_opt(0, 0, 0) {
                let timestamp = date.timestamp() as u64;
                let mut s_ts = format!("{}", timestamp);
                while s_ts.len() < 13 {
                    s_ts.push('0');
                }
                let n_ts = s_ts.parse::<u64>().unwrap();
                Some(n_ts)
            } else {
                None
            }
        }
        None => None,
    }
}

/// Calculates the time difference in milliseconds between two UNIX timestamps.
///
/// # Arguments
///
/// * `from_ts` - The starting UNIX timestamp in milliseconds.
/// * `to_ts` - The ending UNIX timestamp in milliseconds.
///
/// # Returns
///
/// A `u64` representing the time difference in milliseconds.
pub fn get_time_difference(from_ts: u64, to_ts: u64) -> u64 {
    if from_ts > to_ts {
        0 // Return 0 if from_ts is greater than to_ts
    } else {
        let ts = to_ts - from_ts; // Calculate the difference
        ts
    }
}

/// Calculates the open time of a k-line based on its close time and interval.
///
/// # Arguments
///
/// * `close_time` - The closing time of the k-line in milliseconds.
/// * `interval` - The interval of the k-line (e.g., "1m", "5m").
///
/// # Returns
///
/// A `u64` representing the open time of the k-line in milliseconds.
pub fn calculate_kline_open_time(close_time: u64, interval: &str) -> u64 {
    // Convert the interval to seconds
    let interval_seconds = match interval {
        "1m" => 60,
        "5m" => 5 * 60,
        "15m" => 15 * 60,
        // Add more interval cases as needed
        _ => {
            println!("Unsupported interval: {}", interval);
            return 0; // Return 0 if interval is unsupported
        }
    };

    // Calculate the open time by subtracting interval seconds from the close time
    (close_time + 1) - (interval_seconds * 1000)
}

/// Builds a `Duration` representing the interval specified by a string.
///
/// # Arguments
///
/// * `interval` - The interval as a string (e.g., "1m", "5m").
///
/// # Returns
///
/// A `Result<Duration, &'static str>` which is Ok containing the `Duration` if the interval is supported, or an Err with an error message.
pub fn build_interval(interval: &str) -> Result<Duration, &'static str> {
    match interval {
        "1m" => Ok(Duration::from_secs(60)),
        "5m" => Ok(Duration::from_secs(300)),
        "15m" => Ok(Duration::from_secs(900)),
        "1h" => Ok(Duration::from_secs(3600)),
        _ => Err("Unsupported interval"),
    }
}

// TODO: docs
pub fn floor_mili_ts(timestamp_millis: u64, mili_sec: u64) -> u64 {
    let floored_intervals = timestamp_millis / mili_sec;
    floored_intervals * mili_sec
}

/// Converts a Unix timestamp in milliseconds to the start of the month in which it falls.
///
/// # Arguments
///
/// * `open_time` - A Unix timestamp in milliseconds representing the time of interest.
///
/// # Returns
///
/// * A Unix timestamp in milliseconds representing the start of the month.
pub fn floor_month_ts(open_time: u64) -> u64 {
    // Convert the Unix timestamp in milliseconds to a DateTime<Utc>
    let datetime = Utc.timestamp_millis_opt(open_time as i64).unwrap();

    // Construct a new DateTime<Utc> representing the first day of the month at 00:00:00 hours
    let start_of_month = Utc
        .with_ymd_and_hms(datetime.year(), datetime.month(), 1, 0, 0, 0)
        .unwrap();

    // Convert back to a Unix timestamp in milliseconds
    start_of_month.timestamp_millis() as u64
}

// TODO: docs
pub fn add_month_to_timestamp(timestamp: i64) -> i64 {
    let datetime = Utc.timestamp_millis_opt(timestamp).unwrap();
    let next_month = if datetime.month() == 12 {
        Utc.with_ymd_and_hms(datetime.year() + 1, 1, 1, 0, 0, 0)
            .unwrap()
    } else {
        Utc.with_ymd_and_hms(datetime.year(), datetime.month() + 1, 1, 0, 0, 0)
            .unwrap()
    };

    next_month.timestamp_millis()
}

pub fn start_timer() -> Instant {
    Instant::now()
}

/// Calculates the duration between the given `start` instant and the current moment.
/// Returns the duration as `Duration`.
pub fn elapsed_time(start: Instant) -> actix_web::cookie::time::Duration {
    start.elapsed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_date() {
        let year = 2022;
        let month = 1;
        let day = 1;

        let ts = 1640995200000;

        let result = year_month_day_to_ts(year, month, day);

        assert!(result.is_some());

        assert_eq!(result.unwrap(), ts);
    }

    #[test]
    fn test_invalid_date() {
        let year = 2024;
        let month = 2;
        let day = 30;

        let result = year_month_day_to_ts(year, month, day);

        assert!(result.is_none());
    }

    #[test]
    fn test_string_to_timestamp() {
        let ts = 1640995200000;

        let result = string_to_timestamp("2022-01-01T00:00:00Z");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ts);
    }

    #[test]
    fn test_timestamp_to_string() {
        let ts = 1640995200000;
        let str = "2022-01-01T00:00:00Z";

        let result = timestamp_to_string(ts);

        assert_eq!(result, str);
    }

    #[test]
    fn test_timestamp_to_datetime() {
        let ts1 = 1640995200000;
        let ts2 = 1640995200000000;

        let t1 = timestamp_to_datetime(ts1);
        let t2 = timestamp_to_datetime(ts2);

        assert_eq!(t1, t2);
    }

    #[test]
    fn test_generate_ts() {
        // Test the generate_ts function
        let result = generate_ts();

        // Assert that the result is a valid timestamp
        assert!(result > 0);
    }

    #[test]
    fn test_year_month_day_to_ts() {
        let year = 2022;
        let month = 1; // replace with the current month
        let day = 1; // replace with the current day

        let result = year_month_day_to_ts(year, month, day);

        // Assert that the result is Some(u64)
        assert!(result.is_some());

        assert!(result.unwrap() == 1640995200000);
    }

    #[test]
    fn test_year_month_day_to_ts_with_invalid_date() {
        // Test year_month_day_to_ts with an invalid date
        let invalid_year = 2024;
        let invalid_month = 2;
        let invalid_day = 30; // replace with an invalid day

        let result = year_month_day_to_ts(invalid_year, invalid_month, invalid_day);

        // Assert that the result is None for an invalid date
        assert!(result.is_none());
    }
}

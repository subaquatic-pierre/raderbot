use chrono::prelude::DateTime;

use chrono::NaiveDate;
use chrono::Utc;
use chrono::{NaiveDateTime, TimeZone};

use std::time::Duration;
use std::time::SystemTime;

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

pub fn timestamp_to_datetime(timestamp: u64) -> DateTime<Utc> {
    let mut s_ts = format!("{}", timestamp);
    while s_ts.len() < 13 {
        s_ts.push('0');
    }
    let n_ts = s_ts.parse::<u64>().unwrap();
    let naive =
        NaiveDateTime::from_timestamp_opt(n_ts as i64 / 1000, (n_ts % 1000) as u32 * 1_000_000)
            .unwrap();
    DateTime::<Utc>::from_utc(naive, Utc)
}

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

pub fn get_time_difference(from_ts: u64, to_ts: u64) -> u64 {
    if from_ts > to_ts {
        0 // Return 0 if from_ts is greater than to_ts
    } else {
        let ts = to_ts - from_ts; // Calculate the difference
        convert_timestamp(ts);
        ts
    }
}

fn convert_timestamp(timestamp: u64) {
    // Calculate the equivalent time units
    let milliseconds = timestamp % 1000;
    let seconds = (timestamp / 1000) % 60;
    let minutes = (timestamp / (1000 * 60)) % 60;
    let hours = (timestamp / (1000 * 60 * 60)) % 24;
    let days = (timestamp / (1000 * 60 * 60 * 24)) % 365;
    let years = timestamp / (1000 * 60 * 60 * 24 * 365);

    // Print the converted values
    println!("Years: {}", years);
    println!("Days: {}", days);
    println!("Hours: {}", hours);
    println!("Minutes: {}", minutes);
    println!("Seconds: {}", seconds);
    println!("Milliseconds: {}", milliseconds);
}

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

pub fn build_interval(interval: &str) -> Result<Duration, &'static str> {
    match interval {
        "1m" => Ok(Duration::from_secs(60)),
        "5m" => Ok(Duration::from_secs(300)),
        "15m" => Ok(Duration::from_secs(900)),
        "1h" => Ok(Duration::from_secs(3600)),
        _ => Err("Unsupported interval"),
    }
}

use std::{fmt::Display, time::Duration};

use serde::{Deserialize, Serialize};

use crate::utils::time::{DAY_AS_MILI, HOUR_AS_MILI, MIN_AS_MILI};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Interval {
    #[serde(rename = "1m")]
    Min1,
    #[serde(rename = "5m")]
    Min5,
    #[serde(rename = "15m")]
    Min15,
    #[serde(rename = "1h")]
    Hour1,
    #[serde(rename = "1d")]
    Day1,
}

impl Interval {
    pub fn to_duration(&self) -> Duration {
        match self {
            Interval::Min1 => Duration::from_millis(MIN_AS_MILI),
            Interval::Min5 => Duration::from_millis(MIN_AS_MILI * 5),
            Interval::Min15 => Duration::from_millis(MIN_AS_MILI * 15),
            Interval::Hour1 => Duration::from_millis(HOUR_AS_MILI),
            Interval::Day1 => Duration::from_millis(DAY_AS_MILI),
        }
    }

    pub fn to_mili(&self) -> u64 {
        match self {
            Interval::Min1 => MIN_AS_MILI,
            Interval::Min5 => MIN_AS_MILI,
            Interval::Min15 => MIN_AS_MILI * 15,
            Interval::Hour1 => HOUR_AS_MILI,
            Interval::Day1 => DAY_AS_MILI,
        }
    }
}

impl TryFrom<&str> for Interval {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "1m" => Ok(Interval::Min1),
            "5m" => Ok(Interval::Min5),
            "15m" => Ok(Interval::Min15),
            "1h" => Ok(Interval::Hour1),
            "1d" => Ok(Interval::Day1),
            _ => Err("Unable to parse interval"),
        }
    }
}

impl TryFrom<String> for Interval {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interval::Min1 => write!(f, "1m"),
            Interval::Min5 => write!(f, "5m"),
            Interval::Min15 => write!(f, "15m"),
            Interval::Hour1 => write!(f, "1h"),
            Interval::Day1 => write!(f, "1d"),
        }
    }
}

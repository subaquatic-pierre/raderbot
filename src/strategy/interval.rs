use std::{
    fmt::{write, Display},
    time::Duration,
};

use crate::utils::time::{HOUR_AS_MILI, MIN_AS_MILI};

pub enum Interval {
    Min1,
    Min5,
    Min15,
    Hour1,
}

impl Interval {
    fn to_duration(&self) -> Duration {
        match self {
            Interval::Min1 => Duration::from_millis(MIN_AS_MILI),
            Interval::Min5 => Duration::from_millis(MIN_AS_MILI * 5),
            Interval::Min15 => Duration::from_millis(MIN_AS_MILI * 15),
            Interval::Hour1 => Duration::from_millis(HOUR_AS_MILI),
        }
    }

    fn to_mili(&self) -> u64 {
        match self {
            Interval::Min1 => MIN_AS_MILI,
            Interval::Min5 => MIN_AS_MILI,
            Interval::Min15 => MIN_AS_MILI * 15,
            Interval::Hour1 => HOUR_AS_MILI,
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
            _ => Err("Unknown interval string"),
        }
    }
}

impl Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interval::Min1 => write!(f, "1m"),
            Interval::Min5 => write!(f, "5m"),
            Interval::Min15 => write!(f, "15m"),
            Interval::Hour1 => write!(f, "1h"),
        }
    }
}

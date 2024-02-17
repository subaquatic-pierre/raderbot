use rand::Rng;

use std::collections::HashMap;

use crate::exchange::types::ApiError;
use crate::exchange::types::ApiResult;

use serde_json::Value;

pub fn parse_f64_from_lookup(key: &str, lookup: &HashMap<String, Value>) -> ApiResult<f64> {
    let num = lookup
        .get(key)
        .ok_or_else(|| {
            // Create an error message or construct an error type
            "'time' missing from data lookup is missing".to_string()
        })?
        .as_str()
        .ok_or_else(|| {
            // Create an error message or construct an error type
            "Unable to parse as u64".to_string()
        })?
        .parse::<f64>();

    match num {
        Ok(num) => Ok(num),
        Err(e) => Err(ApiError::Parsing(e.to_string())),
    }
}

pub fn generate_random_id() -> u32 {
    let mut rng = rand::thread_rng();
    rng.gen()
}

pub fn gen_random_milliseconds() -> u64 {
    rand::thread_rng().gen_range(1000..3000)
}

use rand::Rng;

use crate::exchange::types::ApiError;
use crate::exchange::types::ApiResult;
use std::collections::HashMap;

use serde_json::Value;

/// Parses a floating-point number from a given lookup HashMap.
///
/// # Arguments
///
/// * `key` - The key to look up in the HashMap.
/// * `lookup` - The HashMap containing string keys and JSON `Value`s.
///
/// # Returns
///
/// Returns an `ApiResult<f64>`, which is Ok containing the parsed number if successful,
/// or an `ApiError` if the key is missing or the value cannot be parsed as a `f64`.
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

/// Parses a usize from a JSON `Value` by a given key.
///
/// # Arguments
///
/// * `key` - The key to look up in the JSON `Value`.
/// * `value` - The JSON `Value` containing the data.
///
/// # Returns
///
/// Returns `Ok(usize)` if the value exists and can be parsed as `usize`,
/// or an error message if the key is missing or the value cannot be parsed.
pub fn parse_usize_from_value(key: &str, value: &Value) -> Result<usize, &'static str> {
    if let Some(val) = value.get(key) {
        if let Some(num) = val.as_u64() {
            return Ok(num as usize);
        }
    }

    Err("Unable to parse usize from value")
}

/// Generates a random ID.
///
/// # Returns
///
/// Returns a `u32` random ID.
pub fn generate_random_id() -> u32 {
    let mut rng = rand::thread_rng();
    rng.gen()
}

/// Generates a random number within the range 1000 to 2999, representing milliseconds.
///
/// # Returns
///
/// Returns a `u64` representing the random milliseconds.
pub fn _gen_random_milliseconds() -> u64 {
    rand::thread_rng().gen_range(1000..3000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Tests parsing a `f64` from a lookup map.
    #[test]
    fn test_parse_f64_from_lookup() {
        let mut lookup = HashMap::new();
        lookup.insert("key".to_string(), json!("123.45"));

        // Test with a valid key
        assert_eq!(
            parse_f64_from_lookup("key", &lookup).unwrap(),
            123.45,
            "Failed to parse f64 from lookup"
        );

        // Test with a missing key
        assert!(parse_f64_from_lookup("non_existing_key", &lookup).is_err());

        // Test with a key pointing to a non-string value
        lookup.insert("key".to_string(), json!(123)); // Insert an integer value
        assert!(parse_f64_from_lookup("key", &lookup).is_err());
    }

    /// Tests parsing a `usize` from a JSON `Value`.
    #[test]
    fn test_parse_usize_from_value() {
        // Test with a valid value containing a usize
        let value = json!({"key": 10});
        assert_eq!(
            parse_usize_from_value("key", &value),
            Ok(10),
            "Failed to parse usize from value"
        );

        // Test with a missing key
        let value = json!({});
        assert!(parse_usize_from_value("non_existing_key", &value).is_err());

        // Test with a key pointing to a non-integer value
        let value = json!({"key": "string_value"});
        assert!(parse_usize_from_value("key", &value).is_err());
    }

    /// Tests the generation of random IDs to ensure they are indeed random.
    #[test]
    fn test_generate_random_id() {
        // Test the generate_random_id function
        let id1 = generate_random_id();
        let id2 = generate_random_id();

        // Assert that the generated IDs are different
        assert_ne!(id1, id2, "Random IDs should not be equal");
    }

    /// Tests the generation of random milliseconds within the specified range.
    #[test]
    fn test_gen_random_milliseconds() {
        // Test the gen_random_milliseconds function
        let milliseconds = _gen_random_milliseconds();

        // Assert that the generated milliseconds are within the expected range
        assert!(milliseconds >= 1000 && milliseconds < 3000);
    }
}

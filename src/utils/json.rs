use flate2::read::GzDecoder;

use serde_json::Value;
use std::io::Read;

pub fn _parse_gzip_to_json(gzip_data: Vec<u8>) -> Result<Value, Box<dyn std::error::Error>> {
    let mut decoder = GzDecoder::new(gzip_data.as_slice());
    let mut json_string = String::new();
    decoder.read_to_string(&mut json_string)?;

    let json: Value = serde_json::from_str(&json_string)?;

    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use serde_json::json;
    use std::io::Write;

    #[test]
    fn test_parse_gzip_to_json() {
        // Test with valid gzip data
        let input_json = json!({"key": "value"});
        let gzip_data = compress_json(&input_json);
        let result = _parse_gzip_to_json(gzip_data.clone());
        assert!(result.is_ok());

        let parsed_json = result.unwrap();
        assert_eq!(parsed_json, input_json);

        // Test with invalid gzip data
        let invalid_gzip_data = vec![1, 2, 3, 4, 5];
        let result = _parse_gzip_to_json(invalid_gzip_data);
        assert!(result.is_err());

        // Add more test cases as needed to cover different scenarios
    }

    // Helper function to compress JSON into gzip data
    fn compress_json(json: &serde_json::Value) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        write!(encoder, "{}", json).unwrap();
        encoder.finish().unwrap()
    }
}

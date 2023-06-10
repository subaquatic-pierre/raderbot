use flate2::read::GzDecoder;
use serde_json::Value;
use std::io::Read;

pub fn parse_gzip_to_json(gzip_data: Vec<u8>) -> Result<Value, Box<dyn std::error::Error>> {
    let mut decoder = GzDecoder::new(gzip_data.as_slice());
    let mut json_string = String::new();
    decoder.read_to_string(&mut json_string)?;

    let json: Value = serde_json::from_str(&json_string)?;

    Ok(json)
}

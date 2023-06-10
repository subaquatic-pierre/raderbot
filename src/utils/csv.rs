use std::fs::File;

use std::io::{BufRead, BufReader, Result};

pub fn has_header(file_path: &str, expected_header: &[&str]) -> Result<bool> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let first_line = reader.lines().next();

    match first_line {
        Some(line) => {
            let header = line?;
            let header_fields: Vec<&str> = header.split(',').collect();
            Ok(header_fields == expected_header)
        }
        None => Ok(false),
    }
}

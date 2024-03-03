use std::fs::File;
use std::io::{BufRead, BufReader, Result};

/// Checks if the specified CSV file contains the expected header.
///
/// This function opens a CSV file and reads the first line to compare it against
/// an expected header provided as a slice of string slices. It is useful for validating
/// CSV file formats before processing their contents.
///
/// # Parameters
///
/// * `file_path`: A `&str` representing the path to the CSV file to check.
/// * `expected_header`: A slice of string slices (`&[&str]`), representing the expected header fields in order.
///
/// # Returns
///
/// A `Result<bool>` indicating whether the file contains the expected header (`true`) or not (`false`).
/// Returns `Err` if the file cannot be opened or if there is an issue reading the first line of the file.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let file_path = "path/to/your/file.csv";
/// let expected_header = &["Column1", "Column2", "Column3"];
/// match has_header(file_path, expected_header) {
///     Ok(has_header) => {
///         if has_header {
///             println!("File contains the expected header.");
///         } else {
///             println!("File does not contain the expected header.");
///         }
///     }
///     Err(e) => println!("Error checking file header: {}", e),
/// }
/// ```
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

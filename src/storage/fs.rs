use csv::ReaderBuilder;
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::Write;
use std::io::{self};
use std::path::{Path, PathBuf};

use crate::market::kline::Kline;
use crate::strategy::strategy::{StrategyId, StrategyInfo, StrategySummary};
use crate::utils::kline::{
    build_kline_filename, build_kline_key, generate_kline_filenames_in_range,
};
use crate::utils::time::generate_ts;

use super::manager::StorageManager;

/// Represents a file system-based storage manager for managing klines and strategy summaries.

#[derive(Serialize, Deserialize, Clone)]
pub struct FsStorageManager {
    app_directory: PathBuf,
    data_directory: PathBuf,
}

impl FsStorageManager {
    /// Creates a new instance of `FsStorageManager` with a specified data directory.
    ///
    /// # Arguments
    ///
    /// * `data_directory` - A path reference that specifies where to store the data.

    pub fn new(data_directory: impl AsRef<Path>) -> Self {
        let app_directory = Self::create_app_directory();
        let data_directory = app_directory.join(data_directory);

        if !data_directory.exists() {
            fs::create_dir_all(&data_directory).expect("Failed to create data directory");
        }

        Self {
            app_directory,
            data_directory,
        }
    }

    /// Loads klines from a specified file.
    ///
    /// # Arguments
    ///
    /// * `filename` - The name of the file to load klines from.
    ///
    /// # Returns
    ///
    /// Returns an `Option` that contains a vector of `Kline` if the file exists and is successfully read; otherwise `None`.

    pub fn _load_klines(&self, filename: &str) -> Option<Vec<Kline>> {
        let mut market_dir = self.data_directory.join("market");
        market_dir.push("klines");
        let file_path = market_dir.join(filename);

        if let Ok(file) = fs::File::open(file_path) {
            let mut reader = ReaderBuilder::new().has_headers(false).from_reader(file);

            let mut klines: Vec<Kline> = Vec::new();

            for result in reader.deserialize() {
                if let Ok(kline) = result {
                    klines.push(kline);
                } else {
                    // Handle error while deserializing kline
                    return None;
                }
            }

            Some(klines)
        } else {
            None
        }
    }

    /// Creates the application directory in the user's home directory if it doesn't already exist.
    ///
    /// # Returns
    ///
    /// Returns the path to the application directory.

    fn create_app_directory() -> PathBuf {
        let user_dirs = UserDirs::new().expect("Failed to get user directories");
        let home_dir = user_dirs.home_dir();
        let app_directory = home_dir.join(".raderbot");

        if !app_directory.exists() {
            fs::create_dir_all(&app_directory).expect("Failed to create application directory");
        }

        app_directory
    }

    /// Builds the file path for storing strategy summary based on the strategy ID.
    ///
    /// # Arguments
    ///
    /// * `strategy_id` - The unique identifier of the strategy.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the file path if successful, or an error if not.

    fn strategy_summary_filepath(
        &self,
        strategy_id: StrategyId,
    ) -> Result<PathBuf, Box<dyn Error>> {
        // Build market directory and subdirectory for klines
        let data_dir = self.data_directory.join("strategies");
        std::fs::create_dir_all(&data_dir)?;
        let filename = format!("{strategy_id}.json");

        let filepath = data_dir.join(filename);

        Ok(filepath)
    }
}

impl Default for FsStorageManager {
    fn default() -> Self {
        // Create the default data directory path
        let app_directory = Self::create_app_directory();
        let data_directory = app_directory.join("default");

        if !data_directory.exists() {
            fs::create_dir_all(&data_directory).expect("Failed to create data directory");
        }

        Self {
            app_directory,
            data_directory,
        }
    }
}

impl StorageManager for FsStorageManager {
    /// Saves klines to the file system.
    ///
    /// # Arguments
    ///
    /// * `klines` - A slice of `Kline` to be saved.
    /// * `kline_key` - A string slice that represents the key associated with the klines.
    ///
    /// # Returns
    ///
    /// Returns an `io::Result<()>` indicating the outcome of the operation.

    fn save_klines(&self, klines: &[Kline], kline_key: &str) -> io::Result<()> {
        // Build market directory and subdirectory for klines
        let mut market_dir = self.data_directory.join("market");
        market_dir.push("klines");
        std::fs::create_dir_all(&market_dir)?;

        for kline in klines {
            // Build file path
            let kline_filename = build_kline_filename(kline_key, kline.open_time);
            let file_path = market_dir.join(kline_filename);

            let file_exists = file_path.exists();

            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&file_path)?;

            let mut writer = csv::WriterBuilder::new()
                .has_headers(false)
                .from_writer(file);

            if file_exists {
                // Read the existing klines from the file
                let mut reader = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .from_path(&file_path)?;

                // Read existing klines into a vector
                let existing_klines: Vec<Kline> =
                    reader.deserialize().collect::<Result<Vec<Kline>, _>>()?;

                if let Some((last_index, last_existing_kline)) =
                    existing_klines.iter().enumerate().last()
                {
                    if last_existing_kline.open_time == kline.open_time {
                        // Overwrite the last entry with the new kline
                        let mut overwrite_file = File::create(&file_path)?;
                        let mut overwrite_writer = csv::WriterBuilder::new()
                            .has_headers(false)
                            .from_writer(&mut overwrite_file);

                        // Write existing klines excluding the last entry
                        for existing_kline in &existing_klines[..last_index] {
                            overwrite_writer.serialize(existing_kline)?;
                        }

                        // Write the new kline
                        overwrite_writer.serialize(kline)?;
                    } else {
                        // Append the new kline to the existing file
                        writer.serialize(kline)?;
                    }
                } else {
                    // Append the new kline to the existing file
                    writer.serialize(kline)?;
                }
            } else {
                // Serialize and write the kline to the file
                writer.serialize(kline)?;
            }
        }

        Ok(())
    }

    /// Retrieves klines based on the specified criteria.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol associated with the klines.
    /// * `interval` - The interval of the klines.
    /// * `from_ts` - Optional start timestamp for filtering.
    /// * `to_ts` - Optional end timestamp for filtering.
    /// * `_limit` - Optional limit on the number of klines to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a vector of `Kline` that match the criteria.

    fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
        _limit: Option<usize>,
    ) -> Vec<Kline> {
        let kline_key = build_kline_key(symbol, interval);

        // create filtered klines to hold all klines which are filtered
        let mut filtered_klines: Vec<Kline> = Vec::new();

        let filenames = match from_ts {
            Some(from_ts) => match to_ts {
                Some(to_ts) => Some(generate_kline_filenames_in_range(
                    &kline_key, from_ts, to_ts,
                )),
                None => Some(generate_kline_filenames_in_range(
                    &kline_key,
                    from_ts,
                    generate_ts(),
                )),
            },
            None => None,
        };

        if let Some(filenames) = filenames {
            for kline_filename in filenames {
                if let Some(klines) = self._load_klines(&kline_filename) {
                    filtered_klines.extend_from_slice(&klines);
                }
            }
        };

        filtered_klines
    }

    /// Saves a strategy summary to the file system.
    ///
    /// # Arguments
    ///
    /// * `summary` - The `StrategySummary` to be saved.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating the outcome of the operation.

    fn save_strategy_summary(&self, summary: StrategySummary) -> Result<(), Box<dyn Error>> {
        let filepath = self.strategy_summary_filepath(summary.info.id)?;
        let json_str = serde_json::to_string(&summary)?;

        // Write JSON string to a file
        let mut file = File::create(filepath)?;
        file.write_all(json_str.as_bytes())?;

        Ok(())
    }

    /// Lists all saved strategy summaries.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `StrategyInfo` if successful, or an error if not.

    fn list_saved_strategies(&self) -> Result<Vec<StrategyInfo>, Box<dyn Error>> {
        let mut data = vec![];

        let data_dir = self.data_directory.join("strategies");

        if data_dir.is_dir() {
            for entry in fs::read_dir(data_dir)? {
                let entry = entry?;
                let path = entry.path();
                if let Some(extension) = path.extension() {
                    if extension == "json" {
                        let file_content = fs::read_to_string(path)?;
                        let strategy_summary: StrategySummary =
                            serde_json::from_str(&file_content)?;
                        data.push(strategy_summary.info);
                    }
                }
            }
        }

        Ok(data)
    }

    /// Retrieves a strategy summary based on a strategy ID.
    ///
    /// # Arguments
    ///
    /// * `strategy_id` - The unique identifier of the strategy.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `StrategySummary` if found, or an error if not.

    fn get_strategy_summary(
        &self,
        strategy_id: StrategyId,
    ) -> Result<StrategySummary, Box<dyn Error>> {
        let filepath = self.strategy_summary_filepath(strategy_id)?;

        let mut file = File::open(filepath)?;

        // Read the contents of the file into a string
        let mut json_str = String::new();
        file.read_to_string(&mut json_str)?;

        // Deserialize the JSON string into a data structure
        let data: StrategySummary = serde_json::from_str(&json_str)?;
        Ok(data)
    }
}

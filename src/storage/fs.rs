use async_trait::async_trait;
use csv::ReaderBuilder;
use directories::UserDirs;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::Write;
use std::io::{self};
use std::path::{Path, PathBuf};

use crate::market::kline::Kline;
use crate::market::trade::Trade;
use crate::strategy::strategy::{StrategyId, StrategyInfo, StrategySummary};
use crate::utils::kline::{
    build_kline_filename, build_kline_key, generate_kline_filenames_in_range, get_min_max_open_time,
};
use crate::utils::time::{floor_mili_ts, floor_month_ts, generate_ts, DAY_AS_MILI};
use crate::utils::trade::{
    build_market_trade_filename, build_market_trade_key, generate_trade_filenames_in_range,
};

use super::manager::StorageManager;

/// Represents a file system-based storage manager for managing klines and strategy summaries.

#[derive(Serialize, Deserialize, Clone)]
pub struct FsStorage {
    app_directory: PathBuf,
    data_directory: PathBuf,
}

impl FsStorage {
    /// Creates a new instance of `FsStorage` with a specified data directory.
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

    fn _load_klines(&self, filename: &str) -> Option<Vec<Kline>> {
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

    // TODO: docs
    fn _load_trades(&self, filename: &str) -> Option<Vec<Trade>> {
        let mut market_dir = self.data_directory.join("market");
        market_dir.push("trades");
        let file_path = market_dir.join(filename);

        if let Ok(file) = fs::File::open(file_path) {
            let mut reader = ReaderBuilder::new().has_headers(false).from_reader(file);

            let mut trades: Vec<Trade> = Vec::new();

            for result in reader.deserialize() {
                if let Ok(kline) = result {
                    trades.push(kline);
                } else {
                    // Handle error while deserializing kline
                    return None;
                }
            }

            Some(trades)
        } else {
            None
        }
    }

    // TODO: docs
    pub fn _merge_klines(&self, existing_klines: &[Kline], fresh_klines: &[Kline]) -> Vec<Kline> {
        let mut merged = Vec::new();

        if let Some(first_fresh) = fresh_klines.first() {
            for existing_kline in existing_klines {
                if existing_kline.open_time < first_fresh.open_time {
                    merged.push(existing_kline.clone())
                } else {
                    break;
                }
            }
            merged.extend_from_slice(fresh_klines);
        } else {
            merged.extend_from_slice(existing_klines);
        }

        merged
    }

    // TODO: docs
    pub fn _merge_trades(&self, existing_trades: &[Trade], fresh_trades: &[Trade]) -> Vec<Trade> {
        let mut merged = Vec::new();

        if let Some(first_fresh) = fresh_trades.first() {
            for existing_kline in existing_trades {
                if existing_kline.timestamp < first_fresh.timestamp {
                    merged.push(existing_kline.clone())
                } else {
                    break;
                }
            }
            merged.extend_from_slice(fresh_trades);
        } else {
            merged.extend_from_slice(existing_trades);
        }

        merged
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

impl Default for FsStorage {
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

#[async_trait]
impl StorageManager for FsStorage {
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

    async fn save_klines(
        &self,
        klines: &[Kline],
        kline_key: &str,
        is_bootstrap: bool,
    ) -> io::Result<()> {
        // Build market directory and subdirectory for klines
        let mut market_dir = self.data_directory.join("market");
        market_dir.push("klines");
        std::fs::create_dir_all(&market_dir)?;

        // sort klines into month buckets
        let mut klines_by_month: HashMap<u64, Vec<Kline>> = HashMap::new();
        for kline in klines {
            let month_ts = floor_month_ts(kline.open_time);

            if let Some(klines) = klines_by_month.get_mut(&month_ts) {
                klines.push(kline.clone());
            } else {
                klines_by_month.insert(month_ts, vec![kline.clone()]);
            }
        }

        for (month_ts, mut klines) in klines_by_month {
            let mut klines_to_save = vec![];

            let kline_filename = build_kline_filename(kline_key, month_ts);
            let file_path = market_dir.join(kline_filename);

            if file_path.exists() {
                let mut reader = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .from_path(&file_path)?;

                // Read existing klines into a vector
                let mut existing_klines: Vec<Kline> =
                    reader.deserialize().collect::<Result<Vec<Kline>, _>>()?;

                // sort klines by open_time
                existing_klines.sort_by_key(|k| k.open_time);
                klines.sort_by_key(|k| k.open_time);

                let merged = self._merge_klines(&existing_klines, &klines);

                klines_to_save.extend_from_slice(&merged);
            } else {
                klines_to_save.extend_from_slice(&klines);
            }

            let file = OpenOptions::new()
                .append(!is_bootstrap)
                .write(true)
                .create(true)
                .open(&file_path)?;

            let mut writer = csv::WriterBuilder::new()
                .has_headers(false)
                .from_writer(file);

            for kline in klines_to_save {
                writer.serialize(kline)?
            }

            writer.flush()?
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

    async fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
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

    async fn save_strategy_summary(&self, summary: StrategySummary) -> Result<(), Box<dyn Error>> {
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

    async fn list_saved_strategies(&self) -> Result<Vec<StrategyInfo>, Box<dyn Error>> {
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

    async fn get_strategy_summary(
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

    // TODO: Docs
    async fn get_trades(
        &self,
        symbol: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
    ) -> Vec<Trade> {
        let trade_key = build_market_trade_key(symbol);

        // create filtered klines to hold all klines which are filtered
        let mut filtered_trades: Vec<Trade> = Vec::new();

        let filenames = match from_ts {
            Some(from_ts) => match to_ts {
                Some(to_ts) => Some(generate_trade_filenames_in_range(
                    &trade_key, from_ts, to_ts,
                )),
                None => Some(generate_trade_filenames_in_range(
                    &trade_key,
                    from_ts,
                    generate_ts(),
                )),
            },
            None => None,
        };

        if let Some(filenames) = filenames {
            for trade_filename in filenames {
                if let Some(trades) = self._load_trades(&trade_filename) {
                    filtered_trades.extend_from_slice(&trades);
                }
            }
        };

        filtered_trades
    }

    // TODO: docs
    async fn save_trades(
        &self,
        trades: &[Trade],
        trade_key: &str,
        _is_bootstrap: bool,
    ) -> io::Result<()> {
        // Build market directory and subdirectory for klines
        let mut market_dir = self.data_directory.join("market");
        market_dir.push("trades");
        std::fs::create_dir_all(&market_dir)?;

        let mut trades_by_day: HashMap<u64, Vec<Trade>> = HashMap::new();
        for trade in trades {
            let ts = floor_mili_ts(trade.timestamp, DAY_AS_MILI);
            if let Some(trades) = trades_by_day.get_mut(&ts) {
                trades.push(trade.clone())
            } else {
                trades_by_day.insert(ts, vec![trade.clone()]);
            }
        }

        for (trade_ts, mut trades) in trades_by_day {
            // Build file path
            let trade_filename = build_market_trade_filename(trade_key, trade_ts);
            let mut trades_to_save = vec![];

            let file_path = market_dir.join(trade_filename);

            if file_path.exists() {
                let mut reader = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .from_path(&file_path)?;

                // Read existing klines into a vector
                let mut existing_trades: Vec<Trade> =
                    reader.deserialize().collect::<Result<Vec<Trade>, _>>()?;

                // sort klines by open_time
                existing_trades.sort_by_key(|k| k.timestamp);
                trades.sort_by_key(|k| k.timestamp);

                let merged = self._merge_trades(&existing_trades, &trades);

                trades_to_save.extend_from_slice(&merged);
            } else {
                trades_to_save.extend_from_slice(&trades);
            }

            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&file_path)?;

            let mut writer = csv::WriterBuilder::new()
                .has_headers(false)
                .from_writer(file);

            for kline in trades_to_save {
                writer.serialize(kline)?
            }

            writer.flush()?
        }

        Ok(())
    }
}

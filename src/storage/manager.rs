use csv::ReaderBuilder;
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{self};
use std::path::{Path, PathBuf};

use crate::market::{kline::Kline, market::MarketData};

#[derive(Serialize, Deserialize, Clone)]
pub struct StorageManager {
    app_directory: PathBuf,
    data_directory: PathBuf,
}

impl StorageManager {
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

    pub fn save_klines(&self, klines: &[Kline], kline_key: &str) -> io::Result<()> {
        // Build market directory and subdirectory for klines
        let mut market_dir = self.data_directory.join("market");
        market_dir.push("klines");
        std::fs::create_dir_all(&market_dir)?;

        for kline in klines {
            // Build file path
            let kline_filename = MarketData::build_kline_filename(kline_key, kline.open_time);
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

    pub fn load_klines(&self, filename: &str) -> Option<Vec<Kline>> {
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

    fn create_app_directory() -> PathBuf {
        let user_dirs = UserDirs::new().expect("Failed to get user directories");
        let home_dir = user_dirs.home_dir();
        let app_directory = home_dir.join(".raderbot");

        if !app_directory.exists() {
            fs::create_dir_all(&app_directory).expect("Failed to create application directory");
        }

        app_directory
    }
}

impl Default for StorageManager {
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

use super::manager::StorageManager;
use crate::{
    account::trade::OrderSide,
    market::{interval::Interval, kline::Kline, trade::Trade},
    strategy::strategy::{StrategyId, StrategyInfo, StrategySummary},
    utils::{
        bson::{build_bson_kline_meta, build_bson_trade_meta},
        kline::build_kline_key,
        time::{elapsed_time, start_timer, timestamp_to_datetime},
        trade::build_market_trade_key,
    },
};
use base64::engine::general_purpose;
use base64::Engine;
use uuid::Uuid;

use async_trait::async_trait;
use futures::{TryFutureExt, TryStreamExt};
use futures_util::StreamExt;
use log::info;
use mongodb::{
    bson::{self, doc, to_document},
    IndexModel,
};
use mongodb::{
    bson::{from_bson, to_bson, Bson},
    error::Error as MongoError,
    options::IndexOptions,
};
use mongodb::{
    bson::{DateTime, Uuid as BsonUuid},
    options::{
        CreateCollectionOptions, DeleteOptions, InsertOneOptions, TimeseriesOptions, UpdateOptions,
    },
};
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{fmt, io};

#[derive(Debug)]
struct MongoErrorWrapper(String);

impl fmt::Display for MongoErrorWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for MongoErrorWrapper {}

pub struct MongoDbStorage {
    client: Client,
}

impl MongoDbStorage {
    pub async fn new(uri: &str) -> Result<Self, Box<dyn Error>> {
        let client = Client::with_uri_str(uri).await?;
        let mut _self = MongoDbStorage { client };

        // _self.init().await.ok();
        Ok(_self)
    }

    async fn kline_collection(
        &self,
        collection_name: &str,
    ) -> Result<Collection<BsonKline>, String> {
        let collection_name = collection_name.replace("@", "_");
        let db = self.client.database("trading_db");
        self.init_timeseries_collection(&collection_name, "open_time", "metadata")
            .await?;
        Ok(db.collection(&collection_name))
    }

    async fn trade_collection(
        &self,
        collection_name: &str,
    ) -> Result<Collection<BsonMarketTrade>, String> {
        let collection_name = collection_name.replace("@", "_");
        let db = self.client.database("trading_db");
        self.init_timeseries_collection(&collection_name, "timestamp", "metadata")
            .await?;
        Ok(db.collection(&collection_name))
    }

    fn strategy_collection(&self) -> Collection<StrategySummary> {
        self.client.database("trading_db").collection("strategy")
    }

    async fn init_timeseries_collection(
        &self,
        collection_name: &str,
        time_field: &str,
        meta_field: &str,
    ) -> Result<(), String> {
        let db = self.client.database("trading_db");
        if !db
            .list_collection_names(None)
            .await
            .map_err(|e| e.to_string())?
            .contains(&collection_name.to_string())
        {
            // Options for creating a time series collection
            let timeseries_options = TimeseriesOptions::builder()
                .time_field(time_field.to_string())
                .meta_field(Some(meta_field.to_string()))
                .granularity(Some(mongodb::options::TimeseriesGranularity::Seconds)) // Specify the field used for time
                .build();

            let create_options = CreateCollectionOptions::builder()
                .timeseries(timeseries_options)
                .build();
            db.create_collection(collection_name, create_options)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

#[async_trait]
impl StorageManager for MongoDbStorage {
    async fn get_klines(
        &self,
        symbol: &str,
        interval: Interval,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
    ) -> Vec<Kline> {
        let collection_name = build_kline_key(symbol, interval);
        let collection = match self.kline_collection(&collection_name).await {
            Ok(collection) => collection,
            Err(e) => {
                info!("{e}");
                return vec![];
            }
        };

        let mut query = doc! {
            "symbol": symbol,
            "interval": interval.to_string(),
        };

        if let Some(from_ts) = from_ts {
            let ts = bson::DateTime::from_millis(from_ts as i64);

            query.insert("open_time", doc! { "$gte": ts });
        }
        if let Some(to_ts) = to_ts {
            let ts = bson::DateTime::from_millis(to_ts as i64);

            query.insert("close_time", doc! { "$lte": ts });
        }

        if let Ok(mut cursor) = collection.find(query, None).await {
            let mut klines: Vec<Kline> = Vec::new();
            while let Some(result) = cursor.next().await {
                if let Ok(bson_kline) = result {
                    klines.push(bson_kline.into());
                }
            }
            return klines;
        }

        vec![]
    }

    async fn save_klines(
        &self,
        klines: &[Kline],
        kline_key: &str,
        is_bootstrap: bool,
    ) -> io::Result<()> {
        let collection = self
            .kline_collection(kline_key)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        if is_bootstrap {
            // delete all existing klines with open_times
            let metas: Vec<String> = klines.iter().map(|k| build_bson_kline_meta(k)).collect();

            let query = doc! {"metadata": {"$in": metas }};
            match collection.delete_many(query, None).await {
                Err(e) => {
                    info!("Error deleting klines before save, e: {e}");
                }
                Ok(res) => {
                    // info!("Deleted klines before save, {res:?}");
                }
            };
        }

        let bson_klines: Vec<BsonKline> = klines.iter().map(|k| k.clone().into()).collect();

        if let Err(e) = collection.insert_many(bson_klines, None).await {
            info!("{e}")
        }

        Ok(())
    }

    // TODO: Docs
    async fn get_trades(
        &self,
        symbol: &str,
        from_ts: Option<u64>,
        to_ts: Option<u64>,
    ) -> Vec<Trade> {
        let mut timestamp_query = doc! {};

        if let Some(from_ts) = from_ts {
            let ts = bson::DateTime::from_millis(from_ts as i64);
            timestamp_query.insert("$gte", ts);
        }
        if let Some(to_ts) = to_ts {
            let ts = bson::DateTime::from_millis(to_ts as i64);
            timestamp_query.insert("$lte", ts);
        }

        let mut query = doc! {
            "symbol": symbol,
        };

        if !timestamp_query.is_empty() {
            query.insert("timestamp", timestamp_query);
        }

        let collection_name = build_market_trade_key(symbol);
        let collection = match self.trade_collection(&collection_name).await {
            Err(e) => {
                info!("{e}");
                return vec![];
            }
            Ok(collection) => collection,
        };

        let mut trades: Vec<Trade> = Vec::new();

        if let Ok(mut cursor) = collection.find(query, None).await {
            while let Ok(Some(trade)) = cursor.try_next().await {
                trades.push(trade.into());
            }
            return trades;
        }

        vec![]
    }

    // TODO: docs
    async fn save_trades(
        &self,
        trades: &[Trade],
        trade_key: &str,
        is_bootstrap: bool,
    ) -> std::io::Result<()> {
        let collection = self
            .trade_collection(trade_key)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let window_size = 500_000;
        let total_len = trades.len();

        let mut cur = 0;
        let mut end = cur + window_size;

        while end < total_len {
            // only perform delete on bootstrap mode
            if is_bootstrap {
                let metas: Vec<String> = trades[cur..end]
                    .iter()
                    .map(|t| build_bson_trade_meta(t))
                    .collect();

                let metas_len = metas.len();
                let query = doc! {"metadata": {"$in": metas}};

                match collection.delete_many(query, None).await {
                    Err(e) => {
                        info!(
                            "Error deleting {} number of metas, inside window, e: {e}",
                            metas_len
                        )
                    }
                    Ok(_res) => {
                        // info!("Deleted inside window, cur: {cur} - end: {end} {res:?}",)
                    }
                };
            }

            let bson_trades: Vec<BsonMarketTrade> =
                trades[cur..end].iter().map(|k| k.clone().into()).collect();
            let trades_len = bson_trades.len();
            if let Err(e) = collection.insert_many(bson_trades, None).await {
                info!(
                    "Error inserting {} number of trades, inside window, e: {e}",
                    trades_len
                )
            }

            cur += window_size;
            end = cur + window_size;
        }

        // only delete on bootstrap mode
        if is_bootstrap {
            let metas: Vec<String> = trades[cur..]
                .iter()
                .map(|t| build_bson_trade_meta(t))
                .collect();
            let metas_len = metas.len();
            info!("Deleting remaining IDS if exist: {}", metas_len);
            let query = doc! {"metadata": {"$in": metas}};

            match collection.delete_many(query, None).await {
                Err(e) => {
                    info!(
                        "Error deleting {} number of ids, remaining metas, e: {e}",
                        metas_len
                    )
                }
                Ok(_res) => {
                    // info!("Deleted remaining trades, cur: {cur} - end: {end} {res:?}",)
                }
            };
        }

        let bson_trades: Vec<BsonMarketTrade> =
            trades[cur..].iter().map(|k| k.clone().into()).collect();
        // info!(
        let trades_len = bson_trades.len();

        // info!(
        //     "Adding remain trades, with length {} ...",
        //     bson_trades.len()
        // );
        if let Err(e) = collection.insert_many(bson_trades, None).await {
            info!(
                "Error inserting {} number of trades, remaining trades, e: {e}",
                trades_len
            )
        }
        Ok(())
    }

    async fn list_saved_strategies(&self) -> Result<Vec<StrategyInfo>, Box<dyn Error>> {
        let collection = self.strategy_collection();

        let mut cursor = collection.find(doc! {}, None).await?;

        let mut infos = vec![];

        while let Some(result) = cursor.next().await {
            if let Ok(strategy) = result {
                infos.push(strategy.info);
            }
        }

        Ok(infos)
    }

    async fn save_strategy_summary(&self, summary: StrategySummary) -> Result<(), Box<dyn Error>> {
        let collection = self.strategy_collection();

        let _ = collection
            .insert_one(summary, None)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>);

        Ok(())
    }

    async fn get_strategy_summary(
        &self,
        strategy_id: StrategyId,
    ) -> Result<StrategySummary, Box<dyn Error>> {
        let collection = self.strategy_collection();

        let encoded = general_purpose::STANDARD.encode(&strategy_id.as_bytes());

        let binary = bson::Binary::from_base64(&encoded, None)?;

        let filter = doc! {"info.id": binary};

        if let Some(res) = collection.find_one(filter, None).await? {
            Ok(res)
        } else {
            Err(Box::new(MongoErrorWrapper("No strategy found".to_string())))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BsonKline {
    pub metadata: String,
    pub symbol: String,
    pub interval: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub open_time: DateTime,
    pub close_time: DateTime,
}

impl From<Kline> for BsonKline {
    fn from(kline: Kline) -> Self {
        BsonKline {
            metadata: build_bson_kline_meta(&kline).to_string(),
            symbol: kline.symbol,
            interval: kline.interval.to_string(),
            open: kline.open,
            high: kline.high,
            low: kline.low,
            close: kline.close,
            volume: kline.volume,
            open_time: DateTime::from_millis(kline.open_time as i64),
            close_time: DateTime::from_millis(kline.close_time as i64),
        }
    }
}

impl From<BsonKline> for Kline {
    fn from(bson_kline: BsonKline) -> Self {
        Kline {
            symbol: bson_kline.symbol,
            interval: bson_kline
                .interval
                .try_into()
                .map_err(|e| "X".to_string())
                .unwrap(),
            open: bson_kline.open,
            high: bson_kline.high,
            low: bson_kline.low,
            close: bson_kline.close,
            volume: bson_kline.volume,
            open_time: bson_kline.open_time.timestamp_millis() as u64,
            close_time: bson_kline.close_time.timestamp_millis() as u64,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BsonMarketTrade {
    pub metadata: String,
    pub symbol: String,
    pub timestamp: DateTime,
    pub qty: f64,
    pub price: f64,
    pub order_side: OrderSide,
}

impl From<Trade> for BsonMarketTrade {
    fn from(trade: Trade) -> Self {
        Self {
            metadata: build_bson_trade_meta(&trade),
            symbol: trade.symbol,
            timestamp: DateTime::from_millis(trade.timestamp as i64),
            qty: trade.qty,
            price: trade.price,
            order_side: trade.order_side,
        }
    }
}

impl From<BsonMarketTrade> for Trade {
    fn from(bson_trade: BsonMarketTrade) -> Self {
        Self {
            symbol: bson_trade.symbol,
            timestamp: bson_trade.timestamp.timestamp_millis() as u64,
            qty: bson_trade.qty,
            price: bson_trade.price,
            order_side: bson_trade.order_side,
        }
    }
}

use std::sync::Arc;

use futures_util::lock::Mutex;
use serde_json::Value;
use tokio::sync::mpsc::unbounded_channel;

use crate::market::types::{ArcReceiver, ArcSender};

pub fn build_arc_channel<T>() -> (ArcSender<T>, ArcReceiver<T>) {
    let (sender, receiver) = unbounded_channel::<T>();

    let receiver = Arc::new(Mutex::new(receiver));
    let sender = Arc::new(sender);

    (sender, receiver)
}

pub async fn get_ticker(url: &str) -> String {
    let client = reqwest::Client::new();

    let response = client.get(url).send().await;

    let response = match response {
        Ok(res) => res.json::<Value>().await.unwrap().to_string(),
        Err(e) => format!("{e:?}"),
    };

    response
}

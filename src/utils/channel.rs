use std::sync::Arc;

use futures_util::lock::Mutex;
use tokio::sync::mpsc::unbounded_channel;

use crate::market::types::{ArcReceiver, ArcSender};

pub fn build_arc_channel<T>() -> (ArcSender<T>, ArcReceiver<T>) {
    let (sender, receiver) = unbounded_channel::<T>();

    let receiver = Arc::new(Mutex::new(receiver));
    let sender = Arc::new(sender);

    (sender, receiver)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_arc_channel() {
        // Test building an ARC channel
        let (sender, receiver) = build_arc_channel::<String>();

        // Send a message through the channel
        sender.send("Test Message".to_string()).unwrap();

        // Receive the message from the channel
        let received_message = receiver.lock().await.recv().await.unwrap();

        // Assert that the sent and received messages match
        assert_eq!(received_message, "Test Message");
    }
}

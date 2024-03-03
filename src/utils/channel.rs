use std::sync::Arc;

use futures_util::lock::Mutex;
use tokio::sync::mpsc::unbounded_channel;

use crate::market::types::{ArcReceiver, ArcSender};

/// Creates a new asynchronous, unbounded channel with both sender and receiver wrapped in `Arc` and `Mutex`.
///
/// This utility function simplifies the creation of channels for message passing in asynchronous contexts,
/// especially when shared ownership and thread-safety are required. The function wraps both ends of the channel
/// (`sender` and `receiver`) in `Arc` and `Mutex` to facilitate safe sharing across threads and async tasks.
///
/// # Type Parameters
///
/// * `T`: The type of messages that can be sent through the channel.
///
/// # Returns
///
/// A tuple containing the `ArcSender<T>` and `ArcReceiver<T>`, which are the sender and receiver
/// sides of the channel respectively.
///
/// # Examples
///
/// ```
/// use my_crate::utils::build_arc_channel;
///
/// #[tokio::main]
/// async fn main() {
///     let (sender, receiver) = build_arc_channel::<String>();
///
///     sender.send("Hello, world!".to_string()).unwrap();
///     // Since the receiver is wrapped in an Arc and Mutex, we need to lock it before awaiting
///     let received = receiver.lock().await.recv().await.unwrap();
///
///     println!("Received: {}", received);
/// }
/// ```
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

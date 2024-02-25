use std::ops::Deref;

use std::sync::Arc;

use futures_util::lock::Mutex;

use serde::Serialize;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// Defines types for thread-safe asynchronous communication channels in Rust.
///
/// Provides wrappers around the standard `UnboundedSender` and `UnboundedReceiver` for use in asynchronous contexts.
pub type ArcSender<T> = Arc<UnboundedSender<T>>;

/// Represents a thread-safe, asynchronously accessible sender part of an unbounded channel.
///
/// This type is an `Arc` wrapper around `tokio::sync::mpsc::UnboundedSender`, allowing it to be shared across threads and tasks safely.
pub type ArcReceiver<T> = Arc<Mutex<UnboundedReceiver<T>>>;

/// A thread-safe, asynchronously lockable wrapper around a shared resource.
///
/// This struct provides synchronized access to the contained value using an `Arc` and a `Mutex`, making it suitable for concurrent programming contexts.
#[derive(Debug)]
pub struct ArcMutex<T>(Arc<Mutex<T>>);

/// Implements serialization for `ArcMutex` wrapped types that are serializable.
///
/// This method allows `ArcMutex` wrapped values to be serialized by first acquiring the lock asynchronously and then serializing the locked value.

impl<T> Serialize for ArcMutex<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let lock = self.0.lock();
        let lock_future = futures::executor::block_on(lock);
        lock_future.serialize(serializer)
    }
}

impl<T> ArcMutex<T> {
    /// Creates a new `ArcMutex` instance, wrapping the provided value with `Arc` and `Mutex` for safe shared access in an asynchronous environment.
    pub fn new(inner: T) -> Self {
        Self(Arc::new(Mutex::new(inner)))
    }
}

/// Implements the `Deref` trait, allowing direct access to the `Mutex` wrapped by the `ArcMutex`.
///
/// This method provides a convenient way to access the underlying `Mutex` without needing to unwrap the `ArcMutex` explicitly.

impl<T> Deref for ArcMutex<T> {
    type Target = Mutex<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Implements the `Clone` trait for `ArcMutex`, enabling the creation of new references to the shared, mutex-protected value.
///
/// Cloning an `ArcMutex` creates a new `Arc` reference to the same underlying mutex-protected value, not a deep copy of the value itself.

impl<T> Clone for ArcMutex<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

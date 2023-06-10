use std::ops::Deref;

use std::sync::Arc;

use futures_util::lock::Mutex;

use serde::Serialize;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub type ArcSender<T> = Arc<UnboundedSender<T>>;
pub type ArcReceiver<T> = Arc<Mutex<UnboundedReceiver<T>>>;

#[derive(Debug)]
pub struct ArcMutex<T>(Arc<Mutex<T>>);

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
    // New method for ArcMutexWrapper
    pub fn new(inner: T) -> Self {
        Self(Arc::new(Mutex::new(inner)))
    }
}

impl<T> Deref for ArcMutex<T> {
    type Target = Mutex<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Clone for ArcMutex<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

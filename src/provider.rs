use chrono::{DateTime, Duration, Utc};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for a shared time provider
pub type SharedTimeProvider = Arc<dyn TimeProvider>;

/// Core trait for time providers
pub trait TimeProvider: Send + Sync {
    /// Get the current time
    fn now(&self) -> DateTime<Utc>;

    /// Wait for the specified duration
    fn wait(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

    /// Wait until the specified deadline
    fn wait_until(&self, deadline: DateTime<Utc>) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

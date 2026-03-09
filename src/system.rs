use crate::provider::TimeProvider;
use chrono::{DateTime, Duration, Utc};
use std::future::Future;
use std::pin::Pin;
use tokio::time;

/// Production time provider that uses actual system time
#[derive(Debug, Clone, Copy)]
pub struct SystemTimeProvider;

impl TimeProvider for SystemTimeProvider {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn wait(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            if let Ok(std_duration) = duration.to_std() {
                time::sleep(std_duration).await;
            }
        })
    }

    fn wait_until(&self, deadline: DateTime<Utc>) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let now = self.now();
            if deadline > now {
                let duration = deadline - now;
                self.wait(duration).await;
            }
        })
    }
}

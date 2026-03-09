use crate::config::TimeSource;
use crate::control::TimeControl;
use crate::provider::SharedTimeProvider;
use crate::test::TestTimeProvider;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

/// Production-safe time provider wrapper that prevents accidental time manipulation
pub struct SafeTimeProvider {
    inner: SharedTimeProvider,
    test_provider: Option<Arc<TestTimeProvider>>,
}

impl SafeTimeProvider {
    /// Create a new SafeTimeProvider from a TimeSource
    pub fn new(source: TimeSource) -> Self {
        match source {
            TimeSource::System => Self {
                inner: Arc::new(crate::system::SystemTimeProvider),
                test_provider: None,
            },
            TimeSource::Test(start) => {
                let test_provider = Arc::new(TestTimeProvider::new(start));
                Self {
                    inner: test_provider.clone() as SharedTimeProvider,
                    test_provider: Some(test_provider),
                }
            },
            TimeSource::TestNow => {
                let test_provider = Arc::new(TestTimeProvider::new_at_now());
                Self {
                    inner: test_provider.clone() as SharedTimeProvider,
                    test_provider: Some(test_provider),
                }
            },
        }
    }
    
    /// Create from an existing test provider (mainly for testing)
    pub fn new_from_test_provider(provider: Arc<TestTimeProvider>) -> Self {
        Self {
            inner: provider.clone() as SharedTimeProvider,
            test_provider: Some(provider),
        }
    }
    
    /// Get the current time
    pub fn now(&self) -> DateTime<Utc> {
        self.inner.now()
    }
    
    /// Wait for the specified duration
    pub async fn wait(&self, duration: Duration) {
        self.inner.wait(duration).await
    }
    
    /// Wait until the specified deadline
    pub async fn wait_until(&self, deadline: DateTime<Utc>) {
        self.inner.wait_until(deadline).await
    }
    
    /// Check if running in test mode
    pub fn is_test_mode(&self) -> bool {
        self.test_provider.is_some()
    }
    
    /// Create an interval that ticks at the specified period.
    /// Each call to `tick()` waits for the period and returns the current time.
    /// The first tick completes immediately.
    pub fn interval(&self, period: Duration) -> Interval {
        Interval {
            provider: self.clone(),
            period,
            first_tick: true,
        }
    }

    /// Get time control for tests (returns None in production)
    ///
    /// This method returns a TimeControl guard that allows time manipulation
    /// only when using a test time provider.
    pub fn test_control(&self) -> Option<TimeControl> {
        self.test_provider
            .as_ref()
            .map(|provider| TimeControl::new(provider.clone()))
    }
}

/// A repeating interval that yields the current time on each tick.
/// Created by [`SafeTimeProvider::interval`].
pub struct Interval {
    provider: SafeTimeProvider,
    period: Duration,
    first_tick: bool,
}

impl Interval {
    /// Wait for the next tick and return the current time.
    /// The first tick completes immediately.
    pub async fn tick(&mut self) -> DateTime<Utc> {
        if self.first_tick {
            self.first_tick = false;
        } else {
            self.provider.wait(self.period).await;
        }
        self.provider.now()
    }
}

impl Clone for SafeTimeProvider {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            test_provider: self.test_provider.clone(),
        }
    }
}
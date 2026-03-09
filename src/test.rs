use crate::provider::TimeProvider;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Notify;

/// Test time provider that allows time manipulation
pub struct TestTimeProvider {
    state: Arc<RwLock<TestState>>,
    /// Notified after each wait() completes, allowing callers to synchronize
    wait_completed: Arc<Notify>,
}

#[derive(Debug)]
struct TestState {
    current_time: DateTime<Utc>,
    total_waited: Duration,
    wait_call_count: usize,
}

impl TestTimeProvider {
    /// Create a new test provider at the specified time
    pub fn new(start: DateTime<Utc>) -> Self {
        Self {
            state: Arc::new(RwLock::new(TestState {
                current_time: start,
                total_waited: Duration::zero(),
                wait_call_count: 0,
            })),
            wait_completed: Arc::new(Notify::new()),
        }
    }

    /// Create a new test provider at the current system time
    pub fn new_at_now() -> Self {
        Self::new(Utc::now())
    }

    /// Advance time by the specified duration
    pub fn advance(&self, duration: Duration) {
        let mut state = self.state.write();
        state.current_time += duration;
    }

    /// Set time to a specific value
    pub fn set(&self, time: DateTime<Utc>) {
        let mut state = self.state.write();
        state.current_time = time;
    }

    /// Get the total duration waited
    pub fn total_waited(&self) -> Duration {
        self.state.read().total_waited
    }

    /// Reset wait tracking statistics
    pub fn reset_wait_tracking(&self) {
        let mut state = self.state.write();
        state.total_waited = Duration::zero();
        state.wait_call_count = 0;
    }

    /// Get the number of wait calls
    pub fn wait_call_count(&self) -> usize {
        self.state.read().wait_call_count
    }

    /// Returns a future that resolves when the next wait() call completes.
    /// Useful for synchronizing test code with spawned tasks that call wait().
    pub async fn wait_completed(&self) {
        self.wait_completed.notified().await;
    }
}

impl TimeProvider for TestTimeProvider {
    fn now(&self) -> DateTime<Utc> {
        self.state.read().current_time
    }

    fn wait(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        {
            let mut state = self.state.write();
            state.current_time += duration;
            state.total_waited += duration;
            state.wait_call_count += 1;
        }

        let notify = self.wait_completed.clone();
        Box::pin(async move {
            // yield to allow other tasks to run
            tokio::task::yield_now().await;
            notify.notify_waiters();
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

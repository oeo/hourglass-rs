use hourglass_rs::{SafeTimeProvider, TestTimeProvider, TimeSource};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

/// a service that runs a job on a schedule.
/// it has no idea whether time is real or fake.
struct Scheduler {
    time: SafeTimeProvider,
    log: Arc<Mutex<Vec<DateTime<Utc>>>>,
}

impl Scheduler {
    fn new(time: SafeTimeProvider) -> Self {
        Self {
            time,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn run(&self, iterations: usize) {
        let mut interval = self.time.interval(Duration::hours(1));
        for _ in 0..iterations {
            let now = interval.tick().await;
            self.log.lock().await.push(now);
        }
    }
}

#[tokio::main]
async fn main() {
    let time = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap()),
    );

    let scheduler = Arc::new(Scheduler::new(time.clone()));

    // spawn the service in the background
    let s = scheduler.clone();
    let handle = tokio::spawn(async move {
        s.run(5).await;
    });

    // wait for it to finish (test time makes this instant)
    handle.await.unwrap();

    // verify the service ran at expected times
    let log = scheduler.log.lock().await;
    assert_eq!(log.len(), 5);

    let start: DateTime<Utc> = "2024-01-01T00:00:00Z".parse().unwrap();
    for (i, entry) in log.iter().enumerate() {
        let expected = start + Duration::hours(i as i64);
        assert_eq!(*entry, expected);
        println!("execution {}: {}", i, entry);
    }

    // demonstrate wait_completed() for synchronizing with spawned tasks.
    // this avoids the common sleep(millis) hack in tests.
    let provider = Arc::new(TestTimeProvider::new(
        "2024-06-01T00:00:00Z".parse().unwrap(),
    ));
    let time2 = SafeTimeProvider::new_from_test_provider(provider.clone());
    let scheduler2 = Arc::new(Scheduler::new(time2));

    let s2 = scheduler2.clone();
    let p = provider.clone();
    tokio::spawn(async move {
        // first tick is immediate, second tick triggers wait()
        s2.run(2).await;
        // notify that the second wait completed
        p.wait_completed().await;
    });

    // wait until the spawned task's wait() call completes
    provider.wait_completed().await;

    let log2 = scheduler2.log.lock().await;
    assert_eq!(log2.len(), 2);
    println!("\nsync demo: captured {} executions via wait_completed()", log2.len());
}

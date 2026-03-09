use hourglass_rs::{SafeTimeProvider, TimeSource};
use chrono::Duration;

#[tokio::main]
async fn main() {
    // production: uses real system time
    let time = SafeTimeProvider::new(TimeSource::System);
    println!("system time: {}", time.now());
    assert!(!time.is_test_mode());

    // test: uses controllable time, starts at a fixed point
    let time = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap()),
    );
    let control = time.test_control().unwrap();

    // wait() advances time instantly in test mode
    let start = time.now();
    time.wait(Duration::days(30)).await;
    assert_eq!(time.now() - start, Duration::days(30));

    // control lets you manipulate time externally
    control.advance(Duration::hours(6));
    control.set("2024-06-01T12:00:00Z".parse().unwrap());
    println!("jumped to: {}", time.now());

    // wait tracking tells you how your code used time
    println!("wait calls: {}, total waited: {} days",
        control.wait_call_count(),
        control.total_waited().num_days(),
    );

    // interval() works like tokio::time::interval but through the provider
    control.reset_wait_tracking();
    let mut interval = time.interval(Duration::hours(1));
    for _ in 0..5 {
        let tick = interval.tick().await;
        println!("tick at: {}", tick);
    }
    // first tick is immediate, so 4 waits for 5 ticks
    assert_eq!(control.wait_call_count(), 4);
    assert_eq!(control.total_waited(), Duration::hours(4));
}

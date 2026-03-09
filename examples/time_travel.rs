use hourglass_rs::{SafeTimeProvider, TimeSource};
use chrono::Duration;

/// demonstrates all time manipulation features.
/// the same business_logic function works identically
/// whether backed by real or test time.
async fn business_logic(time: &SafeTimeProvider) -> i64 {
    let start = time.now();

    // simulate some scheduled work
    time.wait(Duration::hours(2)).await;
    time.wait(Duration::hours(3)).await;

    // wait_until a specific deadline
    let deadline = start + Duration::hours(8);
    time.wait_until(deadline).await;

    let elapsed = time.now() - start;
    elapsed.num_hours()
}

#[tokio::main]
async fn main() {
    let time = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T09:00:00Z".parse().unwrap()),
    );
    let control = time.test_control().unwrap();

    let hours = business_logic(&time).await;

    println!("business logic ran for {} hours", hours);
    println!("current time: {}", time.now());
    println!("wait calls: {}", control.wait_call_count());
    println!("total waited: {} hours", control.total_waited().num_hours());

    assert_eq!(hours, 8);
    assert_eq!(control.wait_call_count(), 3);
    assert_eq!(control.total_waited(), Duration::hours(8));

    // time manipulation after the fact
    control.advance(Duration::days(30));
    println!("\nadvanced 30 days: {}", time.now());

    control.set("2024-12-31T23:59:59Z".parse().unwrap());
    println!("jumped to NYE: {}", time.now());

    // reset tracking for a fresh measurement
    control.reset_wait_tracking();
    time.wait(Duration::seconds(1)).await;
    assert_eq!(control.wait_call_count(), 1);
    assert_eq!(control.total_waited(), Duration::seconds(1));
    println!("\nafter reset: 1 wait call, {} second tracked", control.total_waited().num_seconds());

    // clone shares the same underlying time
    let time2 = time.clone();
    control.advance(Duration::hours(1));
    assert_eq!(time.now(), time2.now());
    println!("clones share time: {}", time.now() == time2.now());
}

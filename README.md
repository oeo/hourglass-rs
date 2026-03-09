# hourglass

A time abstraction for Rust that lets you test time-dependent code by swapping real time for controllable time. Production code uses system time with zero overhead. Test code manipulates time instantly.

## Quick Start

```toml
[dependencies]
hourglass-rs = "0.2.1"
```

```rust
use hourglass_rs::{SafeTimeProvider, TimeSource};
use chrono::Duration;

// production: real system time
let time = SafeTimeProvider::new(TimeSource::System);
println!("now: {}", time.now());
time.wait(Duration::seconds(5)).await; // actually waits

// test: controllable time
let time = SafeTimeProvider::new(
    TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
);
let control = time.test_control().unwrap();

time.wait(Duration::days(30)).await; // returns instantly
assert_eq!(control.wait_call_count(), 1);
assert_eq!(control.total_waited(), Duration::days(30));
```

## Why This Exists

`tokio::time::pause` works if you only need `Instant`/`Duration`. This crate is for when you need:

- `DateTime<Utc>` (chrono) instead of just `Instant`
- Wait call tracking and statistics
- Isolated time per test (not global to the runtime)
- `interval()` that works through the same abstraction

## API

### SafeTimeProvider

The main interface. Pass it to your structs/functions via dependency injection.

```rust
let time = SafeTimeProvider::new(TimeSource::System); // or Test/TestNow

time.now()                          // -> DateTime<Utc>
time.wait(duration).await           // sleep for duration
time.wait_until(deadline).await     // sleep until deadline
time.interval(period)               // -> Interval with tick()
time.is_test_mode()                 // -> bool
time.test_control()                 // -> Option<TimeControl>
time.clone()                        // clones share the same time
```

### TimeControl

Returned by `test_control()` in test mode. Manipulates time and reads statistics.

```rust
let control = time.test_control().unwrap();

control.advance(Duration::hours(6));    // move time forward
control.set(some_datetime);             // jump to specific time
control.total_waited()                  // -> Duration
control.wait_call_count()               // -> usize
control.reset_wait_tracking();          // reset counters
```

### Interval

Created by `time.interval(period)`. First tick is immediate, subsequent ticks wait.

```rust
let mut interval = time.interval(Duration::hours(1));
let t0 = interval.tick().await; // immediate
let t1 = interval.tick().await; // waits 1 hour
```

### TestTimeProvider

Lower-level access. Use `SafeTimeProvider` unless you need `wait_completed()`.

```rust
use hourglass_rs::TestTimeProvider;
use std::sync::Arc;

let provider = Arc::new(TestTimeProvider::new(start_time));
let time = SafeTimeProvider::new_from_test_provider(provider.clone());

// in another task: time.wait(...).await
// synchronize without sleep(millis) hacks:
provider.wait_completed().await;
```

### TimeSource

```rust
TimeSource::System                  // real time (production)
TimeSource::Test(datetime)          // fixed start time
TimeSource::TestNow                 // test mode starting at current time
TimeSource::from_env()?             // from TIME_SOURCE / TIME_START env vars
```

`from_env()` reads:
- `TIME_SOURCE=system` (default) or `TIME_SOURCE=test`
- `TIME_START=2024-01-01T00:00:00Z` (RFC3339, optional)

Returns `Result<TimeSource, TimeSourceError>`.

## Testing Pattern

Write your business logic against `SafeTimeProvider`:

```rust
struct MyService {
    time: SafeTimeProvider,
}

impl MyService {
    async fn run_hourly(&self) {
        let mut interval = self.time.interval(Duration::hours(1));
        loop {
            interval.tick().await;
            self.do_work().await;
        }
    }
}
```

Test it with instant time:

```rust
#[tokio::test]
async fn test_hourly_service() {
    let time = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let control = time.test_control().unwrap();
    let service = MyService { time };

    tokio::spawn(async move { service.run_hourly().await });
    tokio::task::yield_now().await;

    assert_eq!(control.total_waited(), Duration::hours(5));
}
```

## Examples

```bash
cargo run --example basic               # production vs test, interval
cargo run --example time_travel         # all manipulation features
cargo run --example scheduled_service   # realistic service with wait_completed()
```

## Dependencies

- `chrono` - DateTime types
- `tokio` - async runtime (time, rt, macros, sync features)
- `parking_lot` - fast RwLock for test state

## Contributing

Contributions accepted via Pull Request.

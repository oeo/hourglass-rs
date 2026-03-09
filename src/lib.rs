//! # hourglass-rs
//! 
//! A time abstraction crate that provides consistent time handling for both 
//! production and test environments, with safe time manipulation capabilities for testing.
//! 
//! ## Features
//! 
//! - **Zero overhead** in production - thin wrapper around system time
//! - **Time manipulation** in tests - advance time, set specific times
//! - **Async support** - works seamlessly with tokio's async runtime
//! - **Type safety** - can't accidentally manipulate time in production
//! - **Test isolation** - each test gets its own time control
//! 
//! ## Quick Start
//! 
//! ```rust
//! use hourglass_rs::{SafeTimeProvider, TimeSource};
//! use chrono::Duration;
//! 
//! #[tokio::main]
//! async fn main() {
//!     // Production usage
//!     let time = SafeTimeProvider::new(TimeSource::System);
//!     println!("Current time: {}", time.now());
//!     
//!     // Wait for 5 seconds (actually waits in production)
//!     time.wait(Duration::seconds(5)).await;
//! }
//! ```
//! 
//! ## Testing Example
//! 
//! ```rust
//! use hourglass_rs::{SafeTimeProvider, TimeSource};
//! use chrono::Duration;
//! 
//! #[tokio::test]
//! async fn test_time_dependent_code() {
//!     // Create a test time provider
//!     let time = SafeTimeProvider::new(
//!         TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
//!     );
//!     
//!     // Get time control for the test
//!     let control = time.test_control().expect("Should be in test mode");
//!     
//!     // Your time-dependent code
//!     let start = time.now();
//!     time.wait(Duration::days(30)).await; // Returns immediately in tests
//!     let end = time.now();
//!     
//!     // Verify the behavior
//!     assert_eq!(end - start, Duration::days(30));
//!     assert_eq!(control.total_waited(), Duration::days(30));
//! }
//! ```

pub mod config;
pub mod control;
pub mod provider;
pub mod safe;
pub mod system;
pub mod test;

// Re-export main types for convenience
pub use config::{TimeSource, TimeSourceError};
pub use control::TimeControl;
pub use provider::{SharedTimeProvider, TimeProvider};
pub use safe::{Interval, SafeTimeProvider};
pub use system::SystemTimeProvider;
pub use test::TestTimeProvider;

// Re-export chrono types that are part of our API
pub use chrono::{DateTime, Duration, Utc};
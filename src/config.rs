use chrono::{DateTime, Utc};
use std::fmt;

/// Errors that can occur when creating a TimeSource from environment variables
#[derive(Debug)]
pub enum TimeSourceError {
    /// The TIME_START environment variable had an invalid RFC3339 format
    InvalidTimeStart(String),
}

impl fmt::Display for TimeSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeSourceError::InvalidTimeStart(value) => {
                write!(f, "invalid TIME_START format (expected RFC3339): {}", value)
            }
        }
    }
}

impl std::error::Error for TimeSourceError {}

/// Time source configuration for different environments
#[derive(Debug, Clone, Default)]
pub enum TimeSource {
    /// Use system time (production)
    #[default]
    System,
    /// Use test time with initial timestamp
    Test(DateTime<Utc>),
    /// Use test time starting at current system time
    TestNow,
}

impl TimeSource {
    /// Create from environment variables
    /// - TIME_SOURCE: "system" (default) or "test"
    /// - TIME_START: RFC3339 timestamp for test mode start time
    pub fn from_env() -> Result<Self, TimeSourceError> {
        match std::env::var("TIME_SOURCE").as_deref() {
            Ok("test") => {
                if let Ok(start_str) = std::env::var("TIME_START") {
                    match DateTime::parse_from_rfc3339(&start_str) {
                        Ok(start_time) => Ok(TimeSource::Test(start_time.with_timezone(&Utc))),
                        Err(_) => Err(TimeSourceError::InvalidTimeStart(start_str)),
                    }
                } else {
                    Ok(TimeSource::TestNow)
                }
            }
            _ => Ok(TimeSource::System),
        }
    }
}

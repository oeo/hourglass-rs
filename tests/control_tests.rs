use hourglass_rs::{SafeTimeProvider, TimeSource};
use chrono::{DateTime, Duration, Utc};

#[tokio::test]
async fn test_safe_provider_with_system_has_no_control() {
    let provider = SafeTimeProvider::new(TimeSource::System);
    assert!(!provider.is_test_mode());
    assert!(provider.test_control().is_none());
}

#[tokio::test]
async fn test_safe_provider_with_test_has_control() {
    let provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    assert!(provider.is_test_mode());
    assert!(provider.test_control().is_some());
}

#[tokio::test]
async fn test_control_advance_time() {
    let provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let control = provider.test_control().unwrap();
    
    let start = provider.now();
    control.advance(Duration::days(10));
    let end = provider.now();
    
    assert_eq!(end - start, Duration::days(10));
}

#[tokio::test]
async fn test_control_set_time() {
    let provider = SafeTimeProvider::new(TimeSource::TestNow);
    let control = provider.test_control().unwrap();
    
    let new_time = "2025-06-15T12:00:00Z".parse().unwrap();
    control.set(new_time);
    
    assert_eq!(provider.now(), new_time);
}

#[tokio::test]
async fn test_control_wait_tracking() {
    let provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let control = provider.test_control().unwrap();
    
    // Initial state
    assert_eq!(control.total_waited(), Duration::zero());
    assert_eq!(control.wait_call_count(), 0);
    
    // Wait some times
    provider.wait(Duration::hours(1)).await;
    provider.wait(Duration::hours(2)).await;
    provider.wait(Duration::minutes(30)).await;
    
    // Check tracking
    assert_eq!(control.total_waited(), Duration::hours(3) + Duration::minutes(30));
    assert_eq!(control.wait_call_count(), 3);
    
    // Reset and verify
    control.reset_wait_tracking();
    assert_eq!(control.total_waited(), Duration::zero());
    assert_eq!(control.wait_call_count(), 0);
}

#[tokio::test]
async fn test_control_debug_format() {
    let provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let control = provider.test_control().unwrap();
    
    provider.wait(Duration::hours(5)).await;
    
    let debug_str = format!("{:?}", control);
    assert!(debug_str.contains("TimeControl"));
    assert!(debug_str.contains("total_waited"));
    assert!(debug_str.contains("wait_call_count"));
}

#[tokio::test]
async fn test_multiple_controls_share_state() {
    let provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    
    let control1 = provider.test_control().unwrap();
    let control2 = provider.test_control().unwrap();
    
    // Advance time with control1
    control1.advance(Duration::days(5));
    
    // Both provider and control2 should see the change
    assert_eq!(provider.now(), "2024-01-06T00:00:00Z".parse::<DateTime<Utc>>().unwrap());
    
    // Wait with provider
    provider.wait(Duration::days(2)).await;
    
    // Both controls should see the wait statistics
    assert_eq!(control1.total_waited(), Duration::days(2));
    assert_eq!(control2.total_waited(), Duration::days(2));
    assert_eq!(control1.wait_call_count(), 1);
    assert_eq!(control2.wait_call_count(), 1);
}

#[tokio::test]
async fn test_safe_provider_clone_preserves_test_control() {
    let provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let cloned = provider.clone();
    
    assert!(provider.test_control().is_some());
    assert!(cloned.test_control().is_some());
    
    // Both should share the same underlying time
    let control1 = provider.test_control().unwrap();
    let _control2 = cloned.test_control().unwrap();
    
    control1.advance(Duration::hours(3));
    assert_eq!(provider.now(), cloned.now());
}

#[tokio::test]
#[ignore = "Environment variable tests can interfere with each other when run in parallel"]
async fn test_env_source_system() {
    // Set environment variable
    unsafe {
        std::env::set_var("TIME_SOURCE", "system");
    }
    let source = TimeSource::from_env().unwrap();
    let provider = SafeTimeProvider::new(source);
    
    assert!(!provider.is_test_mode());
    assert!(provider.test_control().is_none());
    
    // Clean up
    unsafe {
        std::env::remove_var("TIME_SOURCE");
    }
}

#[tokio::test]
async fn test_env_source_test_with_start() {
    // Set environment variables
    unsafe {
        std::env::set_var("TIME_SOURCE", "test");
        std::env::set_var("TIME_START", "2024-07-04T00:00:00Z");
    }
    
    let source = TimeSource::from_env().unwrap();
    let provider = SafeTimeProvider::new(source);
    
    assert!(provider.is_test_mode());
    assert_eq!(provider.now(), "2024-07-04T00:00:00Z".parse::<DateTime<Utc>>().unwrap());
    
    // Clean up
    unsafe {
        std::env::remove_var("TIME_SOURCE");
        std::env::remove_var("TIME_START");
    }
}

#[tokio::test]
#[ignore = "Environment variable tests can interfere with each other when run in parallel"]
async fn test_env_source_test_without_start() {
    // Set environment variable
    unsafe {
        std::env::set_var("TIME_SOURCE", "test");
    }
    
    let before = chrono::Utc::now();
    let source = TimeSource::from_env().unwrap();
    let provider = SafeTimeProvider::new(source);
    let after = chrono::Utc::now();
    
    assert!(provider.is_test_mode());
    let provider_time = provider.now();
    assert!(provider_time >= before);
    assert!(provider_time <= after);
    
    // Clean up
    unsafe {
        std::env::remove_var("TIME_SOURCE");
    }
}
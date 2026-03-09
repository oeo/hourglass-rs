use hourglass_rs::{SystemTimeProvider, TestTimeProvider, TimeProvider};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

#[tokio::test]
async fn test_system_provider_returns_current_time() {
    let provider = SystemTimeProvider;
    let before = Utc::now();
    let provider_time = provider.now();
    let after = Utc::now();
    
    assert!(provider_time >= before);
    assert!(provider_time <= after);
}

#[tokio::test]
async fn test_system_provider_actually_waits() {
    let provider = SystemTimeProvider;
    let duration = Duration::milliseconds(100);
    
    let start = Utc::now();
    provider.wait(duration).await;
    let end = Utc::now();
    
    let elapsed = end - start;
    assert!(elapsed >= duration);
    assert!(elapsed < duration + Duration::milliseconds(50)); // Allow some margin
}

#[tokio::test]
async fn test_test_provider_advances_time() {
    let start_time = "2024-01-01T00:00:00Z".parse().unwrap();
    let provider = TestTimeProvider::new(start_time);
    
    assert_eq!(provider.now(), start_time);

    let advance_by = Duration::hours(5);
    provider.advance(advance_by);
    
    assert_eq!(provider.now(), start_time + advance_by);
}

#[tokio::test]
async fn test_test_provider_sets_time() {
    let provider = TestTimeProvider::new("2024-01-01T00:00:00Z".parse().unwrap());
    let new_time = "2024-12-25T12:00:00Z".parse().unwrap();
    
    provider.set(new_time);
    assert_eq!(provider.now(), new_time);
}

#[tokio::test]
async fn test_test_provider_wait_advances_time() {
    let provider = TestTimeProvider::new("2024-01-01T00:00:00Z".parse().unwrap());
    let wait_duration = Duration::days(7);
    
    let start = provider.now();
    provider.wait(wait_duration).await;
    let end = provider.now();
    
    assert_eq!(end - start, wait_duration);
}

#[tokio::test]
async fn test_test_provider_wait_until() {
    let start: DateTime<Utc> = "2024-01-01T00:00:00Z".parse().unwrap();
    let provider = TestTimeProvider::new(start);
    let target: DateTime<Utc> = "2024-01-15T00:00:00Z".parse().unwrap();
    
    provider.wait_until(target).await;
    assert_eq!(provider.now(), target);
}

#[tokio::test]
async fn test_test_provider_wait_until_past_does_nothing() {
    let start: DateTime<Utc> = "2024-01-15T00:00:00Z".parse().unwrap();
    let provider = TestTimeProvider::new(start);
    let past_target: DateTime<Utc> = "2024-01-01T00:00:00Z".parse().unwrap();
    
    provider.wait_until(past_target).await;
    assert_eq!(provider.now(), start); // Time should not change
}

#[tokio::test]
async fn test_test_provider_tracks_wait_calls() {
    let provider = TestTimeProvider::new("2024-01-01T00:00:00Z".parse().unwrap());
    
    assert_eq!(provider.wait_call_count(), 0);
    assert_eq!(provider.total_waited(), Duration::zero());
    
    provider.wait(Duration::hours(1)).await;
    provider.wait(Duration::hours(2)).await;
    provider.wait(Duration::hours(3)).await;
    
    assert_eq!(provider.wait_call_count(), 3);
    assert_eq!(provider.total_waited(), Duration::hours(6));
}

#[tokio::test]
async fn test_test_provider_reset_tracking() {
    let provider = TestTimeProvider::new("2024-01-01T00:00:00Z".parse().unwrap());
    
    provider.wait(Duration::hours(5)).await;
    assert_eq!(provider.wait_call_count(), 1);
    assert_eq!(provider.total_waited(), Duration::hours(5));
    
    provider.reset_wait_tracking();
    assert_eq!(provider.wait_call_count(), 0);
    assert_eq!(provider.total_waited(), Duration::zero());
}

#[tokio::test]
async fn test_concurrent_wait_operations() {
    let provider = Arc::new(TestTimeProvider::new("2024-01-01T00:00:00Z".parse().unwrap()));
    
    let p1 = provider.clone();
    let p2 = provider.clone();
    let p3 = provider.clone();
    
    let handle1 = tokio::spawn(async move {
        p1.wait(Duration::hours(1)).await;
        p1.now()
    });
    
    let handle2 = tokio::spawn(async move {
        p2.wait(Duration::hours(2)).await;
        p2.now()
    });
    
    let handle3 = tokio::spawn(async move {
        p3.wait(Duration::hours(3)).await;
        p3.now()
    });
    
    let results = tokio::join!(handle1, handle2, handle3);
    
    // All should complete and show time has advanced
    let time1 = results.0.unwrap();
    let time2 = results.1.unwrap();
    let time3 = results.2.unwrap();
    
    // Due to concurrent execution, we can't predict exact times
    // but all should be at least their wait duration from start
    let start: DateTime<Utc> = "2024-01-01T00:00:00Z".parse().unwrap();
    assert!(time1 >= start + Duration::hours(1));
    assert!(time2 >= start + Duration::hours(2));
    assert!(time3 >= start + Duration::hours(3));
    
    // Total waited should be sum of all waits
    assert_eq!(provider.total_waited(), Duration::hours(6));
    assert_eq!(provider.wait_call_count(), 3);
}

#[tokio::test]
async fn test_thread_safety() {
    let provider = Arc::new(TestTimeProvider::new("2024-01-01T00:00:00Z".parse().unwrap()));
    let mut handles = vec![];
    
    // Spawn many concurrent operations
    for i in 0..10 {
        let p = provider.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..10 {
                p.advance(Duration::minutes(1));
                p.wait(Duration::minutes(1)).await;
                let _ = p.now();
            }
            i
        });
        handles.push(handle);
    }
    
    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Should have advanced time significantly
    let final_time = provider.now();
    let start: DateTime<Utc> = "2024-01-01T00:00:00Z".parse().unwrap();
    assert!(final_time > start);
    
    // Should have recorded all wait calls
    assert_eq!(provider.wait_call_count(), 100); // 10 threads * 10 waits each
}
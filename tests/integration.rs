use hourglass_rs::{SafeTimeProvider, TimeSource};
use chrono::{Duration, DateTime, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Simulated service that depends on time
struct ScheduledService {
    time_provider: SafeTimeProvider,
    executions: Arc<Mutex<Vec<DateTime<Utc>>>>,
}

impl ScheduledService {
    fn new(time_provider: SafeTimeProvider) -> Self {
        Self {
            time_provider,
            executions: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    async fn run_every_hour(&self, duration: Duration) {
        let end_time = self.time_provider.now() + duration;
        
        while self.time_provider.now() < end_time {
            // Record execution
            let now = self.time_provider.now();
            self.executions.lock().await.push(now);
            
            // Wait for next hour
            self.time_provider.wait(Duration::hours(1)).await;
        }
    }
    
    async fn get_execution_count(&self) -> usize {
        self.executions.lock().await.len()
    }
    
    async fn get_executions(&self) -> Vec<DateTime<Utc>> {
        self.executions.lock().await.clone()
    }
}

#[tokio::test]
async fn test_service_with_real_time() {
    // This test demonstrates how the service works with real time
    // We'll use very short durations to keep the test fast
    let provider = SafeTimeProvider::new(TimeSource::System);
    let service = ScheduledService::new(provider.clone());
    
    // Run for a very short duration
    let service_handle = tokio::spawn(async move {
        // Modified to run every 10ms for 50ms (5 executions expected)
        let mut svc = service;
        svc.time_provider = SafeTimeProvider::new(TimeSource::System);
        
        let end_time = svc.time_provider.now() + Duration::milliseconds(50);
        while svc.time_provider.now() < end_time {
            svc.executions.lock().await.push(svc.time_provider.now());
            svc.time_provider.wait(Duration::milliseconds(10)).await;
        }
        svc
    });
    
    let service = service_handle.await.unwrap();
    let count = service.get_execution_count().await;
    
    // Should have executed approximately 5 times (50ms / 10ms)
    assert!(count >= 4 && count <= 6); // Allow some margin for timing
}

#[tokio::test]
async fn test_service_with_test_time() {
    // This test shows how we can test long-running operations instantly
    let provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let control = provider.test_control().unwrap();
    
    let service = Arc::new(ScheduledService::new(provider.clone()));
    
    // Start the service to run for 24 hours
    let service_clone = service.clone();
    let handle = tokio::spawn(async move {
        service_clone.run_every_hour(Duration::hours(24)).await;
    });
    
    // Let the service start
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    
    // Simulate 24 hours passing instantly
    for _hour in 0..24 {
        control.advance(Duration::hours(1));
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    
    // Wait for service to complete
    handle.await.unwrap();
    
    // Verify executions
    let count = service.get_execution_count().await;
    let executions = service.get_executions().await;
    
    assert_eq!(count, 24); // Should have executed exactly 24 times
    
    // Verify execution times
    for (i, execution) in executions.iter().enumerate() {
        let expected = "2024-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap() 
            + Duration::hours(i as i64);
        assert_eq!(*execution, expected);
    }
    
    // Verify time tracking
    assert_eq!(control.total_waited(), Duration::hours(24));
    assert_eq!(control.wait_call_count(), 24);
}

#[tokio::test]
async fn test_concurrent_services_with_test_time() {
    // Create three independent time providers
    let provider1 = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let provider2 = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let provider3 = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    
    // Create multiple services with different schedules and independent time
    let service1 = Arc::new(ScheduledService::new(provider1.clone()));
    let service2 = Arc::new(ScheduledService::new(provider2.clone()));
    let service3 = Arc::new(ScheduledService::new(provider3.clone()));
    
    // Service 1: runs every hour
    let s1 = service1.clone();
    let handle1 = tokio::spawn(async move {
        s1.run_every_hour(Duration::hours(6)).await;
    });
    
    // Service 2: runs every 2 hours
    let s2 = service2.clone();
    let handle2 = tokio::spawn(async move {
        let p = s2.time_provider.clone();
        let end_time = p.now() + Duration::hours(6);
        while p.now() < end_time {
            s2.executions.lock().await.push(p.now());
            p.wait(Duration::hours(2)).await;
        }
    });
    
    // Service 3: runs every 3 hours
    let s3 = service3.clone();
    let handle3 = tokio::spawn(async move {
        let p = s3.time_provider.clone();
        let end_time = p.now() + Duration::hours(6);
        while p.now() < end_time {
            s3.executions.lock().await.push(p.now());
            p.wait(Duration::hours(3)).await;
        }
    });
    
    // Let services start and run
    // Each service has its own isolated time, so they'll complete independently
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    
    // Wait for all services to complete
    // Since each has isolated time, they'll advance through their schedules independently
    tokio::try_join!(handle1, handle2, handle3).unwrap();
    
    // Verify executions
    assert_eq!(service1.get_execution_count().await, 6); // Every hour for 6 hours
    assert_eq!(service2.get_execution_count().await, 3); // Every 2 hours
    assert_eq!(service3.get_execution_count().await, 2); // Every 3 hours
}

#[tokio::test]
async fn test_wait_until_with_concurrent_time_changes() {
    let provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let control = provider.test_control().unwrap();
    
    // Task waiting until a specific time
    let provider_clone = provider.clone();
    let wait_handle = tokio::spawn(async move {
        let target = "2024-01-01T12:00:00Z".parse().unwrap();
        provider_clone.wait_until(target).await;
        provider_clone.now()
    });
    
    // Let the wait start
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    
    // Advance time gradually
    control.advance(Duration::hours(6));
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    
    // Jump directly to target
    control.set("2024-01-01T12:00:00Z".parse().unwrap());
    
    // The wait should complete
    let result = wait_handle.await.unwrap();
    assert_eq!(result, "2024-01-01T12:00:00Z".parse::<DateTime<Utc>>().unwrap());
}

#[tokio::test]
async fn test_mixed_production_and_test_usage() {
    // This shows how the same code can work with both production and test time
    async fn business_logic(provider: &SafeTimeProvider) -> (DateTime<Utc>, DateTime<Utc>) {
        let start = provider.now();
        provider.wait(Duration::hours(8)).await; // Simulate 8 hours of work
        let end = provider.now();
        (start, end)
    }
    
    // Test mode - instant
    let test_provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T09:00:00Z".parse().unwrap())
    );
    let (test_start, test_end) = business_logic(&test_provider).await;
    assert_eq!(test_start, "2024-01-01T09:00:00Z".parse::<DateTime<Utc>>().unwrap());
    assert_eq!(test_end, "2024-01-01T17:00:00Z".parse::<DateTime<Utc>>().unwrap());
    
    // The exact same code would work in production with SystemTimeProvider
    // but would actually wait 8 hours
}

#[tokio::test]
async fn test_interval_ticks() {
    let provider = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T00:00:00Z".parse().unwrap())
    );
    let control = provider.test_control().unwrap();

    let mut interval = provider.interval(Duration::hours(1));

    // First tick is immediate, returns current time
    let t0 = interval.tick().await;
    assert_eq!(t0, "2024-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap());
    assert_eq!(control.wait_call_count(), 0);

    // Subsequent ticks wait for the period
    let t1 = interval.tick().await;
    assert_eq!(t1, "2024-01-01T01:00:00Z".parse::<DateTime<Utc>>().unwrap());
    assert_eq!(control.wait_call_count(), 1);

    let t2 = interval.tick().await;
    assert_eq!(t2, "2024-01-01T02:00:00Z".parse::<DateTime<Utc>>().unwrap());
    assert_eq!(control.wait_call_count(), 2);

    assert_eq!(control.total_waited(), Duration::hours(2));
}
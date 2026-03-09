use hourglass_rs::{SafeTimeProvider, TimeSource};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
struct CollateralTerms {
    margin_call_cvl: f64,  // 125% - Trigger margin call
    liquidation_cvl: f64,  // 105% - Force liquidation
}

#[derive(Debug)]
struct CollateralPosition {
    loan_id: String,
    loan_amount: f64,
    collateral_value: f64,
    terms: CollateralTerms,
    margin_call_sent: Option<DateTime<Utc>>,
    liquidated: bool,
}

impl CollateralPosition {
    fn new(loan_id: String, loan_amount: f64, collateral_value: f64, terms: CollateralTerms) -> Self {
        Self {
            loan_id,
            loan_amount,
            collateral_value,
            terms,
            margin_call_sent: None,
            liquidated: false,
        }
    }
    
    fn cvl_ratio(&self) -> f64 {
        (self.collateral_value / self.loan_amount) * 100.0
    }
    
    fn requires_margin_call(&self) -> bool {
        !self.liquidated && self.cvl_ratio() < self.terms.margin_call_cvl
    }
    
    fn requires_liquidation(&self) -> bool {
        !self.liquidated && self.cvl_ratio() < self.terms.liquidation_cvl
    }
}

#[derive(Clone)]
struct MarginMonitor {
    time_provider: SafeTimeProvider,
    positions: Arc<Mutex<Vec<CollateralPosition>>>,
}

impl MarginMonitor {
    fn new(time_provider: SafeTimeProvider) -> Self {
        Self {
            time_provider,
            positions: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    async fn add_position(&self, position: CollateralPosition) {
        self.positions.lock().await.push(position);
    }
    
    async fn update_collateral_value(&self, loan_id: &str, new_value: f64) {
        let mut positions = self.positions.lock().await;
        if let Some(pos) = positions.iter_mut().find(|p| p.loan_id == loan_id) {
            let old_cvl = pos.cvl_ratio();
            pos.collateral_value = new_value;
            let new_cvl = pos.cvl_ratio();
            
            println!("  Loan {}: CVL changed from {:.1}% to {:.1}%", loan_id, old_cvl, new_cvl);
        }
    }
    
    async fn monitor_positions(&self) {
        let now = self.time_provider.now();
        let mut positions = self.positions.lock().await;
        
        for position in positions.iter_mut() {
            if position.liquidated {
                continue;
            }
            
            let cvl = position.cvl_ratio();
            
            // Check for liquidation
            if position.requires_liquidation() {
                println!("  🔴 Loan {}: LIQUIDATION TRIGGERED (CVL: {:.1}% < {:.1}%)", 
                    position.loan_id, cvl, position.terms.liquidation_cvl);
                position.liquidated = true;
            }
            // Check for margin call
            else if position.requires_margin_call() && position.margin_call_sent.is_none() {
                println!("  ⚠️  Loan {}: MARGIN CALL (CVL: {:.1}% < {:.1}%)", 
                    position.loan_id, cvl, position.terms.margin_call_cvl);
                position.margin_call_sent = Some(now);
            }
            // Healthy position
            else if cvl >= position.terms.margin_call_cvl {
                println!("  ✓  Loan {}: Healthy (CVL: {:.1}%)", position.loan_id, cvl);
            }
        }
    }
    
    async fn run_continuous_monitoring(&self, check_interval: Duration) {
        loop {
            println!("\nMonitoring positions at {}", self.time_provider.now());
            self.monitor_positions().await;
            self.time_provider.wait(check_interval).await;
        }
    }
}

#[tokio::main]
async fn main() {
    println!("=== Margin Monitoring Simulation ===\n");
    
    let time = SafeTimeProvider::new(
        TimeSource::Test("2024-01-01T09:00:00Z".parse().unwrap())
    );
    let control = time.test_control().expect("Should be in test mode");
    
    // Create monitor
    let monitor = MarginMonitor::new(time.clone());
    
    // Create positions with terms from the example
    let terms = CollateralTerms {
        margin_call_cvl: 125.0,
        liquidation_cvl: 105.0,
    };
    
    // Position 1: Starts healthy at 140% CVL
    let position1 = CollateralPosition::new(
        "LOAN-001".to_string(),
        100_000.0,
        140_000.0,  // 140% CVL
        terms.clone(),
    );
    
    // Position 2: Starts healthy at 150% CVL
    let position2 = CollateralPosition::new(
        "LOAN-002".to_string(),
        100_000.0,
        150_000.0,  // 150% CVL
        terms.clone(),
    );
    
    monitor.add_position(position1).await;
    monitor.add_position(position2).await;
    
    println!("Initial positions created:");
    println!("  LOAN-001: $100k loan, $140k collateral (140% CVL)");
    println!("  LOAN-002: $100k loan, $150k collateral (150% CVL)");
    
    // Start monitoring in background
    let monitor_clone = monitor.clone();
    let monitor_handle = tokio::spawn(async move {
        monitor_clone.run_continuous_monitoring(Duration::hours(1)).await;
    });
    
    // Simulate market events
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Hour 1: Small decline
    control.advance(Duration::hours(1));
    monitor.update_collateral_value("LOAN-001", 130_000.0).await;  // 130% CVL
    monitor.update_collateral_value("LOAN-002", 145_000.0).await;  // 145% CVL
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Hour 2: LOAN-001 hits margin call threshold
    control.advance(Duration::hours(1));
    monitor.update_collateral_value("LOAN-001", 120_000.0).await;  // 120% CVL - Margin call!
    monitor.update_collateral_value("LOAN-002", 140_000.0).await;  // 140% CVL
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Hour 3: Market recovers slightly
    control.advance(Duration::hours(1));
    monitor.update_collateral_value("LOAN-001", 122_000.0).await;  // 122% CVL
    monitor.update_collateral_value("LOAN-002", 142_000.0).await;  // 142% CVL
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Hour 4: LOAN-001 crashes to liquidation
    control.advance(Duration::hours(1));
    monitor.update_collateral_value("LOAN-001", 104_000.0).await;  // 104% CVL - Liquidation!
    monitor.update_collateral_value("LOAN-002", 135_000.0).await;  // 135% CVL
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Cancel monitoring
    monitor_handle.abort();
    
    println!("\n=== Final Summary ===");
    let positions = monitor.positions.lock().await;
    for position in positions.iter() {
        println!("Loan {}: CVL {:.1}%, Liquidated: {}, Margin Call: {}", 
            position.loan_id, 
            position.cvl_ratio(),
            position.liquidated,
            position.margin_call_sent.is_some()
        );
    }
    
    println!("\n=== Time Statistics ===");
    println!("Total monitoring duration: {} hours", control.wait_call_count());
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_margin_call_trigger() {
        let time = SafeTimeProvider::new(TimeSource::TestNow);
        let monitor = MarginMonitor::new(time.clone());
        
        let terms = CollateralTerms {
            margin_call_cvl: 125.0,
            liquidation_cvl: 105.0,
        };
        
        let position = CollateralPosition::new(
            "TEST-001".to_string(),
            100_000.0,
            130_000.0,  // 130% CVL - healthy
            terms,
        );
        
        monitor.add_position(position).await;
        
        // Update to trigger margin call
        monitor.update_collateral_value("TEST-001", 120_000.0).await;  // 120% CVL
        monitor.monitor_positions().await;
        
        let positions = monitor.positions.lock().await;
        assert!(positions[0].margin_call_sent.is_some());
        assert!(!positions[0].liquidated);
    }
    
    #[tokio::test]
    async fn test_liquidation_trigger() {
        let time = SafeTimeProvider::new(TimeSource::TestNow);
        let monitor = MarginMonitor::new(time.clone());
        
        let terms = CollateralTerms {
            margin_call_cvl: 125.0,
            liquidation_cvl: 105.0,
        };
        
        let position = CollateralPosition::new(
            "TEST-002".to_string(),
            100_000.0,
            110_000.0,  // 110% CVL
            terms,
        );
        
        monitor.add_position(position).await;
        
        // Update to trigger liquidation
        monitor.update_collateral_value("TEST-002", 100_000.0).await;  // 100% CVL
        monitor.monitor_positions().await;
        
        let positions = monitor.positions.lock().await;
        assert!(positions[0].liquidated);
    }
}
use hourglass_rs::{SafeTimeProvider, TimeSource};
use chrono::{DateTime, Duration, Utc, Datelike, NaiveDate, Timelike};

#[derive(Debug, Clone)]
enum LoanStatus {
    Active,
    PaymentDue,
    Overdue,
}

#[derive(Debug)]
struct LoanLifecycle {
    loan_id: String,
    facility: f64,
    disbursed_at: DateTime<Utc>,
    maturity_date: DateTime<Utc>,
    
    // Accrual tracking
    accrued_interest: f64,
    last_accrual_date: DateTime<Utc>,
    last_cycle_close_date: DateTime<Utc>,
    
    // Payment tracking
    payments: Vec<Payment>,
    
    // Status
    status: LoanStatus,
}

#[derive(Debug)]
struct Payment {
    _date: DateTime<Utc>,
    _amount: f64,
}

impl LoanLifecycle {
    fn new(loan_id: String, facility: f64, disbursed_at: DateTime<Utc>, duration_months: i32) -> Self {
        let maturity_date = Self::add_months(disbursed_at, duration_months);
        
        Self {
            loan_id,
            facility,
            disbursed_at,
            maturity_date,
            accrued_interest: facility * 0.05, // 5% origination fee
            last_accrual_date: disbursed_at,
            last_cycle_close_date: disbursed_at,
            payments: vec![],
            status: LoanStatus::Active,
        }
    }
    
    // Add months handling month-end properly
    fn add_months(date: DateTime<Utc>, months: i32) -> DateTime<Utc> {
        let naive = date.naive_utc();
        let year = naive.year();
        let month = naive.month();
        
        let total_months = month as i32 + months;
        let new_year = year + (total_months - 1) / 12;
        let new_month = ((total_months - 1) % 12 + 1) as u32;
        
        // Handle month-end dates
        let new_day = naive.day().min(Self::days_in_month(new_year, new_month));
        
        NaiveDate::from_ymd_opt(new_year, new_month, new_day)
            .unwrap()
            .and_hms_opt(naive.hour(), naive.minute(), naive.second())
            .unwrap()
            .and_utc()
    }
    
    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if Self::is_leap_year(year) { 29 } else { 28 },
            _ => panic!("Invalid month"),
        }
    }
    
    fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
    
    fn is_month_end(&self, date: DateTime<Utc>) -> bool {
        let tomorrow = date + Duration::days(1);
        date.month() != tomorrow.month()
    }
    
    fn daily_interest_rate(&self) -> f64 {
        0.12 / 365.0  // 12% annual rate
    }
    
    fn accrue_daily_interest(&mut self, date: DateTime<Utc>) {
        let days = (date - self.last_accrual_date).num_days();
        if days > 0 {
            let interest = self.facility * self.daily_interest_rate() * days as f64;
            self.accrued_interest += interest;
            self.last_accrual_date = date;
        }
    }
    
    fn process_month_end(&mut self, date: DateTime<Utc>) {
        if self.is_month_end(date) && date > self.last_cycle_close_date {
            println!("  Month-end cycle close on {}", date.format("%Y-%m-%d"));
            println!("    Interest accrued this cycle: ${:.2}", self.accrued_interest);
            
            self.last_cycle_close_date = date;
            
            // Update status based on payment due
            match self.status {
                LoanStatus::Active => {
                    self.status = LoanStatus::PaymentDue;
                    println!("    Status: Payment Due (0 days grace period)");
                }
                _ => {}
            }
        }
    }
    
    fn make_payment(&mut self, amount: f64, date: DateTime<Utc>) {
        self.payments.push(Payment {
            _date: date,
            _amount: amount,
        });
        
        self.accrued_interest = (self.accrued_interest - amount).max(0.0);
        
        if matches!(self.status, LoanStatus::PaymentDue | LoanStatus::Overdue) {
            if self.accrued_interest == 0.0 {
                self.status = LoanStatus::Active;
            }
        }
    }
    
    fn update_overdue_status(&mut self, date: DateTime<Utc>) {
        if matches!(self.status, LoanStatus::PaymentDue) {
            let days_since_due = (date - self.last_cycle_close_date).num_days();
            if days_since_due > 50 {
                self.status = LoanStatus::Overdue;
                println!("  ⚠️  Loan {} is now OVERDUE (>50 days)", self.loan_id);
            }
        }
    }
}

async fn simulate_loan_lifecycle(time: SafeTimeProvider) {
    // Create loan on Jan 31st (month-end)
    let loan_start = "2024-01-31T00:00:00Z".parse().unwrap();
    let control = time.test_control().expect("Should be in test mode");
    control.set(loan_start);
    
    let mut loan = LoanLifecycle::new(
        "LOAN-EDGE-001".to_string(),
        100_000.0,
        time.now(),
        3,  // 3 month loan
    );
    
    println!("Loan created on: {}", loan.disbursed_at.format("%Y-%m-%d"));
    println!("Maturity date: {}", loan.maturity_date.format("%Y-%m-%d"));
    println!("Initial accrued (5% fee): ${:.2}\n", loan.accrued_interest);
    
    // Simulate day by day
    while time.now() <= loan.maturity_date + Duration::days(60) {
        let today = time.now();
        
        // Daily accrual
        loan.accrue_daily_interest(today);
        
        // Month-end processing
        loan.process_month_end(today);
        
        // Check for overdue
        loan.update_overdue_status(today);
        
        // Simulate payments on specific dates
        if today == "2024-02-29T00:00:00Z".parse::<DateTime<Utc>>().unwrap() {
            println!("\n💰 Making payment on {}", today.format("%Y-%m-%d"));
            let payment_amount = loan.accrued_interest;
            println!("  Paying ${:.2} to cover accrued interest", payment_amount);
            loan.make_payment(payment_amount, today);
        }
        
        // Skip payment in March to trigger overdue
        
        // Advance to next day
        time.wait(Duration::days(1)).await;
    }
    
    println!("\n=== Final Loan State ===");
    println!("Status: {:?}", loan.status);
    println!("Total payments: {}", loan.payments.len());
    println!("Outstanding interest: ${:.2}", loan.accrued_interest);
}

#[tokio::main]
async fn main() {
    println!("=== Loan Lifecycle Edge Cases ===\n");
    
    let time = SafeTimeProvider::new(TimeSource::TestNow);
    
    // Test 1: Month-end loan disbursement
    println!("Test 1: Loan disbursed on month-end (Jan 31)\n");
    simulate_loan_lifecycle(time.clone()).await;
    
    // Test 2: February edge case (28 vs 29 days)
    println!("\n\nTest 2: February payment timing\n");
    let time2 = SafeTimeProvider::new(
        TimeSource::Test("2024-02-01T00:00:00Z".parse().unwrap())
    );
    let control2 = time2.test_control().unwrap();
    
    let mut loan2 = LoanLifecycle::new(
        "LOAN-FEB-001".to_string(),
        100_000.0,
        time2.now(),
        1,
    );
    
    // Simulate through February
    for day in 1..=29 {
        loan2.accrue_daily_interest(time2.now());
        loan2.process_month_end(time2.now());
        
        if day == 28 {
            println!("Feb 28: Is month end? {}", loan2.is_month_end(time2.now()));
        }
        if day == 29 {
            println!("Feb 29 (leap year): Is month end? {}", loan2.is_month_end(time2.now()));
        }
        
        control2.advance(Duration::days(1));
    }
    
    println!("\nTotal interest accrued in February: ${:.2}", loan2.accrued_interest);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_months_edge_cases() {
        // Jan 31 + 1 month = Feb 29 (leap year)
        let jan31 = "2024-01-31T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let feb29 = LoanLifecycle::add_months(jan31, 1);
        assert_eq!(feb29.day(), 29);
        assert_eq!(feb29.month(), 2);
        
        // Jan 31 + 1 month = Feb 28 (non-leap year)
        let jan31_2023 = "2023-01-31T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let feb28 = LoanLifecycle::add_months(jan31_2023, 1);
        assert_eq!(feb28.day(), 28);
        assert_eq!(feb28.month(), 2);
        
        // March 31 + 1 month = April 30
        let mar31 = "2024-03-31T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let apr30 = LoanLifecycle::add_months(mar31, 1);
        assert_eq!(apr30.day(), 30);
        assert_eq!(apr30.month(), 4);
    }
    
    #[tokio::test]
    async fn test_overdue_transition() {
        let time = SafeTimeProvider::new(
            TimeSource::Test("2024-01-31T00:00:00Z".parse().unwrap())
        );
        let control = time.test_control().unwrap();
        
        let mut loan = LoanLifecycle::new("TEST".to_string(), 100_000.0, time.now(), 3);
        
        // Process month end
        loan.process_month_end(time.now());
        assert!(matches!(loan.status, LoanStatus::PaymentDue));
        
        // Advance 50 days - should still be PaymentDue
        control.advance(Duration::days(50));
        loan.update_overdue_status(time.now());
        assert!(matches!(loan.status, LoanStatus::PaymentDue));
        
        // Advance 1 more day - should be Overdue
        control.advance(Duration::days(1));
        loan.update_overdue_status(time.now());
        assert!(matches!(loan.status, LoanStatus::Overdue));
    }
}
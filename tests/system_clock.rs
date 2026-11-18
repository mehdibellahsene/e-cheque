//! Tests de [`e_cheque::SystemClock`].

use chrono::Local;
use e_cheque::{Clock, SystemClock};

#[test]
fn today_returns_local_date() {
    // Arrange
    let before = Local::now().date_naive();

    // Act
    let today = SystemClock.today();

    // Assert
    let after = Local::now().date_naive();
    assert!(
        before <= today && today <= after,
        "{today} hors de [{before}, {after}]"
    );
}

//! Tests de [`e_cheque::PosCommunicator`].
//!
//! Le port série et l'horloge sont remplacés par des doublures `mockall` : des stubs pour
//! simuler les réponses de la caisse et la date du jour, des mocks pour vérifier les échanges.

use std::io::{self, Read, Write};

use chrono::NaiveDate;
use e_cheque::{ChequeData, PosCommunicator, PosError};
use mockall::mock;

mock! {
    Port {}

    impl Read for Port {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    }

    impl Write for Port {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
        fn flush(&mut self) -> io::Result<()>;
    }
}

mock! {
    Clock {}

    impl e_cheque::Clock for Clock {
        fn today(&self) -> NaiveDate;
    }
}

const COMMAND: &[u8] = b"GET_CHEQUE_DATA\n";

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("date de test valide")
}

/// Date du jour simulée.
fn today() -> NaiveDate {
    date(2026, 11, 18)
}

/// Stub de port : accepte toute écriture, renvoie `response` puis signale la fin du flux.
fn port_answering(response: &'static [u8]) -> MockPort {
    let mut port = MockPort::new();
    port.expect_write().returning(|buf| Ok(buf.len()));
    port.expect_flush().returning(|| Ok(()));
    let mut remaining = response;
    port.expect_read().returning(move |buf| remaining.read(buf));
    port
}

/// Stub de port : accepte toute écriture mais échoue à chaque lecture avec `kind`.
fn port_failing_read(kind: io::ErrorKind) -> MockPort {
    let mut port = MockPort::new();
    port.expect_write().returning(|buf| Ok(buf.len()));
    port.expect_flush().returning(|| Ok(()));
    port.expect_read().returning(move |_| Err(kind.into()));
    port
}

/// Stub d'horloge renvoyant toujours `day`.
fn clock_on(day: NaiveDate) -> MockClock {
    let mut clock = MockClock::new();
    clock.expect_today().return_const(day);
    clock
}

/// Client dialoguant avec une caisse qui répond `response`, à la date du jour simulée.
fn pos_answering(response: &'static [u8]) -> PosCommunicator<MockPort, MockClock> {
    PosCommunicator::new(port_answering(response), clock_on(today()))
}

// --- Cas nominaux -------------------------------------------------------------------------

#[test]
fn fetch_with_valid_frame_returns_cheque_data() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    let expected = ChequeData {
        date: date(2026, 11, 16),
        amount_cents: 14_550,
    };
    assert_eq!(result.unwrap(), expected);
}

#[test]
fn fetch_sends_get_cheque_data_command_once() {
    // Arrange
    let mut port = MockPort::new();
    port.expect_write()
        .withf(|buf| buf == COMMAND)
        .times(1)
        .returning(|buf| Ok(buf.len()));
    port.expect_flush().times(1).returning(|| Ok(()));
    let mut response: &[u8] = b"OK;2026-11-16;145.50\n";
    port.expect_read().returning(move |buf| response.read(buf));
    let mut pos = PosCommunicator::new(port, clock_on(today()));

    // Act
    let result = pos.fetch_cheque_data();

    // Assert (les attentes du mock sont vérifiées à sa destruction)
    assert!(result.is_ok(), "résultat inattendu : {result:?}");
}

#[test]
fn fetch_with_crlf_terminator_returns_cheque_data() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;145.50\r\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().amount_cents, 14_550);
}

#[test]
fn fetch_retries_read_after_interruption() {
    // Arrange
    let mut port = MockPort::new();
    port.expect_write().returning(|buf| Ok(buf.len()));
    port.expect_flush().returning(|| Ok(()));
    let mut interrupted = false;
    let mut response: &[u8] = b"OK;2026-11-16;145.50\n";
    port.expect_read().returning(move |buf| {
        if interrupted {
            response.read(buf)
        } else {
            interrupted = true;
            Err(io::ErrorKind::Interrupted.into())
        }
    });
    let mut pos = PosCommunicator::new(port, clock_on(today()));

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().amount_cents, 14_550);
}

// --- Réponses d'erreur de la caisse -------------------------------------------------------

#[test]
fn fetch_with_no_transaction_returns_no_transaction_error() {
    // Arrange
    let mut pos = pos_answering(b"ERR_NO_TRANSACTION\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::NoTransaction)),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_busy_returns_busy_error() {
    // Arrange
    let mut pos = pos_answering(b"ERR_BUSY\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::Busy)),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_unknown_error_code_returns_malformed_frame() {
    // Arrange
    let mut pos = pos_answering(b"ERR_UNKNOWN\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::MalformedFrame(_))),
        "résultat inattendu : {result:?}"
    );
}

// --- Communication ------------------------------------------------------------------------

#[test]
fn fetch_with_read_timeout_returns_timeout_error() {
    // Arrange
    let mut pos = PosCommunicator::new(
        port_failing_read(io::ErrorKind::TimedOut),
        clock_on(today()),
    );

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::Timeout)),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_read_failure_returns_io_error() {
    // Arrange
    let mut pos = PosCommunicator::new(
        port_failing_read(io::ErrorKind::BrokenPipe),
        clock_on(today()),
    );

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::Io(ref e)) if e.kind() == io::ErrorKind::BrokenPipe),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_write_failure_returns_io_error_without_reading() {
    // Arrange
    let mut port = MockPort::new();
    port.expect_write()
        .returning(|_| Err(io::ErrorKind::BrokenPipe.into()));
    port.expect_read().never();
    let mut pos = PosCommunicator::new(port, clock_on(today()));

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::Io(ref e)) if e.kind() == io::ErrorKind::BrokenPipe),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_flush_failure_returns_io_error_without_reading() {
    // Arrange
    let mut port = MockPort::new();
    port.expect_write().returning(|buf| Ok(buf.len()));
    port.expect_flush()
        .returning(|| Err(io::ErrorKind::BrokenPipe.into()));
    port.expect_read().never();
    let mut pos = PosCommunicator::new(port, clock_on(today()));

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::Io(_))),
        "résultat inattendu : {result:?}"
    );
}

// --- Trames vides, corrompues ou incomplètes ----------------------------------------------

#[test]
fn fetch_with_empty_line_returns_empty_response_error() {
    // Arrange
    let mut pos = pos_answering(b"\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::EmptyResponse)),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_closed_stream_and_no_data_returns_empty_response_error() {
    // Arrange
    let mut pos = pos_answering(b"");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::EmptyResponse)),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_frame_missing_newline_returns_malformed_frame() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;145.50");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::MalformedFrame(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_wrong_status_returns_malformed_frame() {
    // Arrange
    let mut pos = pos_answering(b"KO;2026-11-16;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::MalformedFrame(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_lowercase_status_returns_malformed_frame() {
    // Arrange
    let mut pos = pos_answering(b"ok;2026-11-16;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::MalformedFrame(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_missing_field_returns_malformed_frame() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::MalformedFrame(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_extra_field_returns_malformed_frame() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;145.50;EUR\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::MalformedFrame(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_non_utf8_bytes_returns_malformed_frame() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;\xFF\xFE\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::MalformedFrame(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_endless_stream_returns_malformed_frame() {
    // Arrange
    let mut port = MockPort::new();
    port.expect_write().returning(|buf| Ok(buf.len()));
    port.expect_flush().returning(|| Ok(()));
    port.expect_read().returning(|buf| {
        buf[0] = b'A';
        Ok(1)
    });
    let mut pos = PosCommunicator::new(port, clock_on(today()));

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::MalformedFrame(_))),
        "résultat inattendu : {result:?}"
    );
}

// --- Dates ----------------------------------------------------------------------------------

#[test]
fn fetch_with_today_date_returns_cheque_data() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-18;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().date, today());
}

#[test]
fn fetch_with_tomorrow_date_returns_invalid_date() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-19;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidDate(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_compares_date_with_injected_clock() {
    // Arrange
    let mut pos = PosCommunicator::new(
        port_answering(b"OK;2090-01-01;145.50\n"),
        clock_on(date(2099, 12, 31)),
    );

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().date, date(2090, 1, 1));
}

#[test]
fn fetch_with_very_old_date_returns_cheque_data() {
    // Arrange
    let mut pos = pos_answering(b"OK;1970-01-01;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().date, date(1970, 1, 1));
}

#[test]
fn fetch_with_leap_day_of_leap_year_returns_cheque_data() {
    // Arrange
    let mut pos = pos_answering(b"OK;2024-02-29;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().date, date(2024, 2, 29));
}

#[test]
fn fetch_with_leap_day_of_common_year_returns_invalid_date() {
    // Arrange
    let mut pos = pos_answering(b"OK;2025-02-29;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidDate(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_day_out_of_month_returns_invalid_date() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-04-31;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidDate(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_month_thirteen_returns_invalid_date() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-13-01;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidDate(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_day_zero_returns_invalid_date() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-00;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidDate(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_unpadded_date_returns_invalid_date() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-1-5;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidDate(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_signed_year_returns_invalid_date() {
    // Arrange
    let mut pos = pos_answering(b"OK;+2026-11-16;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidDate(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_french_date_format_returns_invalid_date() {
    // Arrange
    let mut pos = pos_answering(b"OK;16/11/2026;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidDate(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_padded_date_returns_invalid_date() {
    // Arrange
    let mut pos = pos_answering(b"OK; 2026-11-16;145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidDate(_))),
        "résultat inattendu : {result:?}"
    );
}

// --- Montants -------------------------------------------------------------------------------

#[test]
fn fetch_with_one_decimal_amount_returns_tens_of_cents() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;145.5\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().amount_cents, 14_550);
}

#[test]
fn fetch_with_integer_amount_returns_whole_cents() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;145\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().amount_cents, 14_500);
}

#[test]
fn fetch_with_smallest_amount_returns_one_cent() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;0.01\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().amount_cents, 1);
}

#[test]
fn fetch_with_largest_amount_returns_max_cents() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;184467440737095516.15\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().amount_cents, u64::MAX);
}

#[test]
fn fetch_with_overflowing_amount_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;184467440737095516.16\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_zero_amount_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;0.00\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_negative_amount_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;-145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_signed_amount_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;+145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_three_decimals_amount_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;145.505\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_comma_separator_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;145,50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_trailing_point_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;145.\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_leading_point_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_leading_zero_amount_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;0145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_zero_units_amount_returns_cents() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;0.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert_eq!(result.unwrap().amount_cents, 50);
}

#[test]
fn fetch_with_empty_amount_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16;\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

#[test]
fn fetch_with_padded_amount_returns_invalid_amount() {
    // Arrange
    let mut pos = pos_answering(b"OK;2026-11-16; 145.50\n");

    // Act
    let result = pos.fetch_cheque_data();

    // Assert
    assert!(
        matches!(result, Err(PosError::InvalidAmount(_))),
        "résultat inattendu : {result:?}"
    );
}

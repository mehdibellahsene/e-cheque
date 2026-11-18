//! Tests de la CLI `e-cheque`, exécutée comme un processus.

use std::process::{Command, Output};

/// Lance la CLI avec `args` et attend sa fin.
fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_e-cheque"))
        .args(args)
        .output()
        .expect("lancement de la CLI")
}

#[test]
fn cli_without_arguments_prints_usage_and_exits_with_2() {
    // Arrange
    let args = [];

    // Act
    let output = run_cli(&args);

    // Assert
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage : e-cheque <PORT> [DEBIT]"));
}

#[test]
fn cli_with_invalid_baud_rate_exits_with_2() {
    // Arrange
    let args = ["port-inexistant-e-cheque", "rapide"];

    // Act
    let output = run_cli(&args);

    // Assert
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("débit invalide"));
}

#[test]
fn cli_with_unknown_port_reports_error_and_exits_with_1() {
    // Arrange
    let args = ["port-inexistant-e-cheque"];

    // Act
    let output = run_cli(&args);

    // Assert
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("erreur du port série"));
    assert!(output.stdout.is_empty());
}

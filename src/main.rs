//! Interroge la caisse sur un port série et affiche les données du chèque en cours.
//!
//! Usage : `e-cheque <PORT> [DEBIT]`, avec un débit de 9600 bauds par défaut.

use std::env;
use std::process::ExitCode;
use std::time::Duration;

use e_cheque::PosCommunicator;

const USAGE: &str = "usage : e-cheque <PORT> [DEBIT]";
const DEFAULT_BAUD_RATE: u32 = 9600;
const TIMEOUT: Duration = Duration::from_secs(2);

fn main() -> ExitCode {
    let mut args = env::args().skip(1);

    let Some(port_name) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let baud_rate = match args.next().map(|raw| raw.parse()) {
        None => DEFAULT_BAUD_RATE,
        Some(Ok(baud_rate)) => baud_rate,
        Some(Err(_)) => {
            eprintln!("débit invalide\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let result = PosCommunicator::open(&port_name, baud_rate, TIMEOUT)
        .and_then(|mut pos| pos.fetch_cheque_data());

    match result {
        Ok(cheque) => {
            println!("Date    : {}", cheque.date);
            println!(
                "Montant : {}.{:02} €",
                cheque.amount_cents / 100,
                cheque.amount_cents % 100
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("erreur : {error}");
            ExitCode::FAILURE
        }
    }
}

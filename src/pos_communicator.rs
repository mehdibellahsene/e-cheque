//! Communication avec la caisse (POS) pour récupérer les données du chèque en cours.
//!
//! Le périphérique dialogue avec la caisse via une liaison série selon le protocole suivant :
//!
//! - requête : `GET_CHEQUE_DATA\n` ;
//! - réponse nominale : `OK;YYYY-MM-DD;MONTANT\n` ;
//! - réponses d'erreur : `ERR_NO_TRANSACTION\n` et `ERR_BUSY\n`.

use std::io::{self, Read, Write};
use std::time::Duration;

use chrono::{Local, NaiveDate};
use serialport::SerialPort;
use thiserror::Error;

/// Commande envoyée à la caisse pour obtenir les données du chèque.
const GET_CHEQUE_DATA_COMMAND: &[u8] = b"GET_CHEQUE_DATA\n";

/// Taille maximale d'une réponse de la caisse, en octets.
const MAX_RESPONSE_LEN: usize = 256;

/// Données du chèque transmises par la caisse, une fois validées.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChequeData {
    /// Date de la transaction.
    pub date: NaiveDate,
    /// Montant de la transaction, en centimes.
    pub amount_cents: u64,
}

/// Erreurs pouvant survenir lors de la récupération des données du chèque.
#[derive(Debug, Error)]
pub enum PosError {
    /// Le port série n'a pas pu être ouvert ou configuré.
    #[error("erreur du port série : {0}")]
    Serial(#[from] serialport::Error),

    /// Une erreur d'entrée/sortie est survenue pendant l'échange.
    #[error("erreur d'entrée/sortie : {0}")]
    Io(#[from] io::Error),

    /// La caisse n'a pas répondu dans le délai imparti.
    #[error("délai d'attente dépassé : aucune réponse de la caisse")]
    Timeout,

    /// Aucun paiement par chèque n'est en cours.
    #[error("aucune transaction par chèque en cours")]
    NoTransaction,

    /// La caisse est occupée.
    #[error("la caisse est occupée")]
    Busy,

    /// La caisse a renvoyé une réponse vide.
    #[error("réponse vide de la caisse")]
    EmptyResponse,

    /// La trame reçue ne respecte pas le protocole.
    #[error("trame malformée : {0}")]
    MalformedFrame(String),

    /// La date reçue est invalide.
    #[error("date invalide : {0}")]
    InvalidDate(String),

    /// Le montant reçu est invalide.
    #[error("montant invalide : {0}")]
    InvalidAmount(String),
}

/// Source de la date du jour, injectée pour rendre la validation des dates déterministe.
pub trait Clock {
    /// Retourne la date du jour.
    fn today(&self) -> NaiveDate;
}

/// Horloge système, dans le fuseau horaire local.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> NaiveDate {
        Local::now().date_naive()
    }
}

/// Client de communication avec la caisse.
///
/// Le canal de communication `P` (tout flux d'octets [`Read`] + [`Write`], comme un port série)
/// et l'horloge `C` sont injectés, ce qui permet de les remplacer par des doublures en test.
pub struct PosCommunicator<P, C = SystemClock> {
    port: P,
    clock: C,
}

impl PosCommunicator<Box<dyn SerialPort>, SystemClock> {
    /// Ouvre le port série `port_name` avec le débit `baud_rate` et le délai de lecture `timeout`,
    /// et utilise l'horloge système.
    ///
    /// # Errors
    ///
    /// Retourne [`PosError::Serial`] si le port ne peut pas être ouvert.
    pub fn open(port_name: &str, baud_rate: u32, timeout: Duration) -> Result<Self, PosError> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(timeout)
            .open()?;

        Ok(Self::new(port, SystemClock))
    }
}

impl<P: Read + Write, C: Clock> PosCommunicator<P, C> {
    /// Crée un client communiquant via `port` et datant les contrôles avec `clock`.
    ///
    /// # Exemple
    ///
    /// Le canal peut être n'importe quel flux d'octets, par exemple un stub en mémoire :
    ///
    /// ```
    /// use std::io::{self, Read, Write};
    ///
    /// use chrono::NaiveDate;
    /// use e_cheque::{Clock, PosCommunicator};
    ///
    /// struct StubPort(&'static [u8]);
    ///
    /// impl Read for StubPort {
    ///     fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    ///         self.0.read(buf)
    ///     }
    /// }
    ///
    /// impl Write for StubPort {
    ///     fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    ///         Ok(buf.len())
    ///     }
    ///
    ///     fn flush(&mut self) -> io::Result<()> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct FixedClock;
    ///
    /// impl Clock for FixedClock {
    ///     fn today(&self) -> NaiveDate {
    ///         NaiveDate::from_ymd_opt(2026, 11, 18).unwrap()
    ///     }
    /// }
    ///
    /// let mut pos = PosCommunicator::new(StubPort(b"OK;2026-11-16;145.50\n"), FixedClock);
    /// let cheque = pos.fetch_cheque_data()?;
    ///
    /// assert_eq!(cheque.amount_cents, 14_550);
    /// # Ok::<(), e_cheque::PosError>(())
    /// ```
    pub fn new(port: P, clock: C) -> Self {
        Self { port, clock }
    }

    /// Demande à la caisse les données du chèque en cours, puis les valide.
    ///
    /// # Errors
    ///
    /// - [`PosError::NoTransaction`] ou [`PosError::Busy`] si la caisse le signale ;
    /// - [`PosError::Timeout`] si la caisse ne répond pas à temps ;
    /// - [`PosError::EmptyResponse`] ou [`PosError::MalformedFrame`] si la trame est invalide ;
    /// - [`PosError::InvalidDate`] ou [`PosError::InvalidAmount`] si les données sont incorrectes ;
    /// - [`PosError::Io`] en cas d'erreur de communication.
    pub fn fetch_cheque_data(&mut self) -> Result<ChequeData, PosError> {
        self.send_command()?;
        let response = self.read_response()?;
        parse_response(&response, &self.clock)
    }

    /// Envoie la commande `GET_CHEQUE_DATA` à la caisse.
    fn send_command(&mut self) -> Result<(), PosError> {
        self.port.write_all(GET_CHEQUE_DATA_COMMAND)?;
        self.port.flush()?;
        Ok(())
    }

    /// Lit la réponse de la caisse jusqu'au caractère `\n`, exclu.
    fn read_response(&mut self) -> Result<String, PosError> {
        let mut buffer = Vec::with_capacity(64);
        let mut byte = [0u8; 1];

        loop {
            match self.port.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    if byte[0] == b'\n' {
                        break;
                    }
                    buffer.push(byte[0]);
                    if buffer.len() > MAX_RESPONSE_LEN {
                        return Err(PosError::MalformedFrame("réponse trop longue".to_string()));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::TimedOut => return Err(PosError::Timeout),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(PosError::Io(e)),
            }
        }

        let response = String::from_utf8(buffer)
            .map_err(|_| PosError::MalformedFrame("réponse non UTF-8".to_string()))?;

        Ok(response.trim_end_matches('\r').to_string())
    }
}

/// Analyse la réponse de la caisse et en extrait les données du chèque.
fn parse_response(response: &str, clock: &impl Clock) -> Result<ChequeData, PosError> {
    if response.is_empty() {
        return Err(PosError::EmptyResponse);
    }

    match response {
        "ERR_NO_TRANSACTION" => return Err(PosError::NoTransaction),
        "ERR_BUSY" => return Err(PosError::Busy),
        _ => {}
    }

    let fields: Vec<&str> = response.split(';').collect();
    if fields.len() != 3 || fields[0] != "OK" {
        return Err(PosError::MalformedFrame(response.to_string()));
    }

    let date = parse_date(fields[1], clock)?;
    let amount_cents = parse_amount(fields[2])?;

    Ok(ChequeData { date, amount_cents })
}

/// Valide une date au format `YYYY-MM-DD`, qui ne doit pas être dans le futur.
fn parse_date(raw: &str, clock: &impl Clock) -> Result<NaiveDate, PosError> {
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|_| PosError::InvalidDate(raw.to_string()))?;

    if date > clock.today() {
        return Err(PosError::InvalidDate(format!("{raw} est dans le futur")));
    }

    Ok(date)
}

/// Convertit un montant décimal (au plus deux décimales) en centimes, sans flottant.
fn parse_amount(raw: &str) -> Result<u64, PosError> {
    let invalid = || PosError::InvalidAmount(raw.to_string());

    let (units, decimals) = raw.split_once('.').unwrap_or((raw, ""));

    if units.is_empty()
        || decimals.len() > 2
        || !units.bytes().all(|b| b.is_ascii_digit())
        || !decimals.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid());
    }

    let units: u64 = units.parse().map_err(|_| invalid())?;
    let cents: u64 = match decimals.len() {
        0 => 0,
        1 => decimals.parse::<u64>().map_err(|_| invalid())? * 10,
        _ => decimals.parse().map_err(|_| invalid())?,
    };

    let amount = units
        .checked_mul(100)
        .and_then(|value| value.checked_add(cents))
        .ok_or_else(invalid)?;

    if amount == 0 {
        return Err(invalid());
    }

    Ok(amount)
}

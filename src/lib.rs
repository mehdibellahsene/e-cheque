//! Communication avec la caisse (POS) pour la lecture des données de chèque.
//!
//! [`PosCommunicator::fetch_cheque_data`] interroge la caisse, valide sa réponse et renvoie un
//! [`ChequeData`], ou une [`PosError`] explicite.

pub mod pos_communicator;

pub use pos_communicator::{ChequeData, Clock, PosCommunicator, PosError, SystemClock};

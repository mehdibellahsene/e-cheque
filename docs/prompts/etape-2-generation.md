# Prompt de génération du module de communication POS

Prompt soumis à un LLM pour générer le module `src/pos_communicator.rs`.
Le code obtenu est intégré au projet **sans modification**.

---

Tu es un développeur Rust expérimenté. Écris le module Rust `pos_communicator` (un seul fichier,
`src/pos_communicator.rs`) utilisé par un périphérique lecteur de chèques pour récupérer auprès
de la caisse (POS) les données du chèque en cours, via une liaison série.

## Protocole

- Requête envoyée par le périphérique : `GET_CHEQUE_DATA\n`
- Réponse nominale de la caisse : `OK;YYYY-MM-DD;MONTANT\n` (exemple : `OK;2026-11-16;145.50\n`)
- Réponses d'erreur de la caisse :
  - `ERR_NO_TRANSACTION\n` : aucun paiement par chèque en cours ;
  - `ERR_BUSY\n` : caisse occupée.
- Toute trame malformée, vide, ou l'absence de réponse (timeout) est une erreur.

## Contraintes techniques

- Rust édition 2024.
- Crates autorisées : `serialport` 4 (liaison série), `chrono` 0.4 (dates), `thiserror` 2 (erreurs).
  Aucune autre dépendance.
- Pas de code `unsafe`, pas de `unwrap()`, `expect()` ni `panic!`.

## API attendue

- `pub struct ChequeData { pub date: chrono::NaiveDate, pub amount_cents: u64 }`,
  dérivant `Debug`, `Clone`, `PartialEq` et `Eq`.
- `pub enum PosError`, défini avec `thiserror`, avec une variante explicite par cas d'erreur :
  port série, entrée/sortie, timeout, aucune transaction, caisse occupée, réponse vide,
  trame malformée, date invalide, montant invalide.
- `pub struct PosCommunicator` exposant :
  - `pub fn new(port_name: &str, baud_rate: u32, timeout: Duration) -> Result<Self, PosError>`,
    qui ouvre le port série ;
  - `pub fn fetch_cheque_data(&mut self) -> Result<ChequeData, PosError>`, qui :
    1. envoie la commande ;
    2. lit la réponse jusqu'au `\n` ;
    3. valide les données (date, montant) ;
    4. retourne un `ChequeData`, ou une `PosError` explicite.

## Règles de validation

- **Trame** : exactement trois champs séparés par `;`, le premier valant `OK`.
- **Date** : format `YYYY-MM-DD`, date calendaire valide, non postérieure à la date du jour.
- **Montant** : nombre décimal avec `.` comme séparateur et au plus deux décimales
  (`145.50`, `145.5`, `145`), strictement positif, converti en centimes sans passer par un
  nombre flottant.

## Livrable

Le code complet du module, documenté en rustdoc (en français), sans tests.

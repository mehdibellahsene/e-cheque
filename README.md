# e-cheque

[![CI](https://github.com/mehdibellahsene/e-cheque/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/mehdibellahsene/e-cheque/actions/workflows/ci.yml)
[![Licence : MIT OR Apache-2.0](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue.svg)](#licence)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-orange.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)

Module de communication avec la caisse (POS) permettant à un périphérique de lecture de chèques
de récupérer la date et le montant de la transaction en cours.

La démarche de test et les choix techniques sont détaillés dans le
[rapport de l'atelier](docs/rapport-atelier-2.pdf).

## Protocole

Le périphérique dialogue avec la caisse via une liaison série (port COM / flux d'octets).

| Sens                  | Trame                    | Signification                          |
| --------------------- | ------------------------ | -------------------------------------- |
| Périphérique → caisse | `GET_CHEQUE_DATA\n`      | Demande des données du chèque          |
| Caisse → périphérique | `OK;YYYY-MM-DD;MONTANT\n` | Réponse nominale (ex. `OK;2026-11-16;145.50\n`) |
| Caisse → périphérique | `ERR_NO_TRANSACTION\n`   | Aucun paiement par chèque en cours     |
| Caisse → périphérique | `ERR_BUSY\n`             | Caisse occupée                         |

Toute trame malformée, vide ou absente (timeout) est traitée comme une erreur.

## Démarrage rapide

Prérequis : [Rust](https://www.rust-lang.org/tools/install) 1.85 ou plus récent.

```sh
git clone https://github.com/mehdibellahsene/e-cheque.git
cd e-cheque
cargo run -- <PORT> [DEBIT]   # ex. /dev/ttyUSB0 ou COM3, 9600 bauds par défaut
```

```text
Date    : 2026-11-16
Montant : 145.50 €
```

## Développement

| Action          | Commande                                       |
| --------------- | ---------------------------------------------- |
| Compiler        | `cargo build`                                  |
| Tester          | `cargo test`                                   |
| Formater        | `cargo fmt --all`                              |
| Analyser        | `cargo clippy --all-targets -- -D warnings`    |
| Documentation   | `cargo doc --no-deps --open`                   |

L'intégration continue ([`ci.yml`](.github/workflows/ci.yml)) exécute ces vérifications sur
Linux, Windows et macOS à chaque pull request et à chaque push sur `main`.

### Couverture de code

La couverture des lignes et des branchements est mesurée avec
[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov), qui requiert la chaîne nightly
pour les branchements :

```sh
rustup toolchain install nightly --component llvm-tools-preview
cargo install cargo-llvm-cov --locked

cargo +nightly llvm-cov --branch --ignore-filename-regex 'src/main\.rs' --open
```

`src/main.rs` est exclu : c'est le code d'assemblage de la CLI, sans logique métier. La CI
échoue si la couverture des lignes passe sous 100 % et publie le rapport HTML en artefact.

### Tests de mutation

La robustesse des tests est évaluée par mutation avec
[cargo-mutants](https://mutants.rs/) : chaque mutant (opérateur inversé, corps de fonction
remplacé…) doit être détecté par au moins un test.

```sh
cargo install cargo-mutants --locked

cargo mutants
```

Le détail est écrit dans `mutants.out/`. Chaque mutant survivant fait l'objet d'une issue
(modèle « Mutant survivant »), fermée par le commit qui renforce les tests. La CI échoue si un
mutant survit.

## Structure

```text
.
├── src/
│   ├── lib.rs               # Racine de la bibliothèque
│   ├── pos_communicator.rs  # Dialogue avec la caisse (PosCommunicator)
│   └── main.rs              # CLI : interroge la caisse et affiche le chèque
├── tests/            # Tests en boîte noire, un fichier par module testé
├── docs/            # Rapport de l'atelier et prompts de génération de code
├── .github/          # CI, Dependabot, modèles d'issue et de pull request
└── Cargo.toml
```

## Contribuer

Les contributions sont les bienvenues. Lisez le [guide de contribution](CONTRIBUTING.md) et le
[code de conduite](CODE_OF_CONDUCT.md) avant d'ouvrir une issue ou une pull request.
Pour signaler une vulnérabilité, suivez la [politique de sécurité](SECURITY.md).

## Licence

Ce projet est distribué sous la licence de votre choix parmi :

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

Sauf mention contraire explicite, toute contribution soumise pour inclusion dans ce projet,
telle que définie par la licence Apache-2.0, est distribuée sous cette double licence, sans
termes ni conditions supplémentaires.

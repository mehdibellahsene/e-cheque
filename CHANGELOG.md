# Journal des modifications

Toutes les modifications notables de ce projet sont documentées dans ce fichier.

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet respecte
le [versionnage sémantique](https://semver.org/lang/fr/).

## [Non publié]

### Ajouté

- CLI `e-cheque <PORT> [DEBIT]` : interroge la caisse et affiche la date et le montant du chèque.
- Module `pos_communicator` : `PosCommunicator::fetch_cheque_data` interroge la caisse et
  valide la date et le montant. Code généré par un LLM à partir de
  [`docs/prompts/etape-2-generation.md`](docs/prompts/etape-2-generation.md).
- Injection des dépendances de `PosCommunicator` : canal de communication générique
  (`Read + Write`) et horloge (`Clock`, implémentée par `SystemClock`).
  `PosCommunicator::open` ouvre le port série réel.
- Suite de tests de `PosCommunicator` et `SystemClock` avec des doublures `mockall`.
- Mesure de la couverture des lignes et des branchements (`cargo-llvm-cov`), avec un seuil de
  100 % des lignes en CI.
- Tests en boîte noire dans `tests/`, structurés selon le modèle AAA.
- Intégration continue : formatage, clippy, tests multiplateformes, MSRV et documentation.
- Dependabot pour les dépendances Cargo et les actions GitHub.
- Documentation communautaire : README, guide de contribution, code de conduite, politique de sécurité.
- Double licence MIT OR Apache-2.0.

### Corrigé

- Une trame sans `\n` final est rejetée au lieu d'être acceptée tronquée.
- Les dates hors gabarit strict `YYYY-MM-DD` (`2026-1-5`, `+2026-11-16`) sont rejetées.
- Les montants `145.` et `0145.50` sont rejetés.

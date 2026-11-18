# Journal des modifications

Toutes les modifications notables de ce projet sont documentées dans ce fichier.

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet respecte
le [versionnage sémantique](https://semver.org/lang/fr/).

## [Non publié]

### Ajouté

- Squelette du crate : bibliothèque et binaire hello world.
- Module `pos_communicator` : `PosCommunicator::fetch_cheque_data` interroge la caisse et
  valide la date et le montant. Code généré par un LLM à partir de
  [`docs/prompts/etape-2-generation.md`](docs/prompts/etape-2-generation.md).
- Injection des dépendances de `PosCommunicator` : canal de communication générique
  (`Read + Write`) et horloge (`Clock`, implémentée par `SystemClock`).
  `PosCommunicator::open` ouvre le port série réel.
- Tests en boîte noire dans `tests/`, structurés selon le modèle AAA.
- Intégration continue : formatage, clippy, tests multiplateformes, MSRV et documentation.
- Dependabot pour les dépendances Cargo et les actions GitHub.
- Documentation communautaire : README, guide de contribution, code de conduite, politique de sécurité.
- Double licence MIT OR Apache-2.0.

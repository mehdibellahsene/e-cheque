# Rapport — Atelier 2 : tests unitaires

Ce rapport résume la démarche suivie, les choix faits à chaque étape et leur justification.
Chaque étape correspond à une pull request, dont les commits détaillent le raisonnement.

## Objectif

Faire générer par un LLM un module qui dialogue avec la caisse, puis **prouver par des tests**
qu'il fonctionne :

1. le rendre testable en isolant ses dépendances (port série, horloge) ;
2. écrire une suite de tests unitaires complète, avec des doublures (stubs et mocks) ;
3. mesurer la couverture du code par ces tests ;
4. vérifier la solidité des tests par mutation.

## Le module testé

Le lecteur de chèques demande à la caisse, via une liaison série, les données du chèque en cours :

```text
Lecteur ── GET_CHEQUE_DATA\n ──────────────▶ Caisse
Lecteur ◀── OK;2026-11-16;145.50\n ──────── Caisse   (ou ERR_NO_TRANSACTION / ERR_BUSY)
```

`PosCommunicator::fetch_cheque_data()` envoie la commande, lit la réponse, valide la date et le
montant, et renvoie un `ChequeData` (date + montant en centimes) ou une `PosError` explicite.

## Étape 2 — Génération par un LLM (#3)

- **Prompt** : [`docs/prompts/etape-2-generation.md`](prompts/etape-2-generation.md).
- **Choix : un prompt précis**, qui impose les bibliothèques (`serialport`, `chrono`,
  `thiserror`), l'API et les règles de validation.
  - Montant en **centimes entiers** (`u64`) : un nombre à virgule flottante ne représente pas
    exactement `145.50`, ce qui est inacceptable pour de l'argent.
  - **Erreurs typées** (`PosError`) : chaque cas d'échec a sa variante, que l'appelant et les
    tests peuvent distinguer.
- Le code produit a été intégré **sans modification**, comme demandé.

## Étape 3 — Isoler les dépendances (#5)

**Analyse** : deux couplages forts empêchaient de tester le module.

| Couplage | Problème |
| --- | --- |
| Ouverture directe du port série dans le constructeur | Impossible de tester sans matériel |
| Lecture de l'horloge système pour refuser une date future | Le résultat change selon le jour |

**Choix : l'injection de dépendances** (inversion de contrôle). Le module ne crée plus ses
dépendances, il les reçoit : `PosCommunicator::new(port, horloge)`.

- **Port** : tout flux d'octets (`Read + Write`). Le faux port des tests n'a qu'à fournir des
  octets, et la lecture jusqu'au `\n` reste dans le module, donc testée. Alternatives écartées :
  un trait « ligne par ligne » (la lecture octet par octet serait sortie du code testé) ou le
  trait `SerialPort` de la bibliothèque (une vingtaine de méthodes à simuler).
- **Horloge** : un trait `Clock` (`today()`), implémenté par `SystemClock` en production et par
  une date fixe en test.
- **Production** : `PosCommunicator::open(port, débit, délai)` assemble le vrai port et
  l'horloge système.

## Étape 4 — Tests unitaires (#6)

- **Framework de mocking** : [`mockall`](https://docs.rs/mockall), la référence en Rust.
  - **Stubs** : le faux port renvoie une réponse prédéfinie, la fausse horloge une date fixe.
  - **Mocks** : on vérifie en plus les échanges, par exemple que `GET_CHEQUE_DATA\n` est envoyée
    exactement une fois, et qu'on ne lit rien si l'envoi a échoué.
- **Organisation** (conventions du cours) : tests en boîte noire dans `tests/`, un fichier par
  composant testé, un cas par fonction de test, structure **Arrange / Act / Assert**.
- **Cas couverts** : nominaux, erreurs de la caisse, erreurs de communication (timeout,
  interruption), trames vides, incomplètes ou corrompues, **dates spéciales** (29 février
  bissextile ou non, 31 avril, jour même, lendemain) et **montants extrêmes** (1 centime,
  valeur maximale, dépassement, zéro, négatif…).
- **Démarche TDD** : les tests décrivent le comportement attendu. Six d'entre eux échouaient
  sur le code généré (phase rouge), corrigé ensuite en trois commits (phase verte).
- **Règles choisies là où le protocole est muet** :
  - une trame sans `\n` final est rejetée (réponse tronquée) ;
  - la date doit suivre exactement `YYYY-MM-DD` (`2026-1-5` est refusé) ;
  - les montants ambigus `145.` et `0145.50` sont refusés ;
  - seule une date **future** est refusée, même très ancienne (fidèle à la spécification).

## Étape 5 — Couverture de code (#7)

- **Outil** : `cargo-llvm-cov`, qui mesure les lignes **et les branchements** (`if`/`else`).
  La mesure des branchements requiert la chaîne Rust nightly.
- **Départ** : 89 % des lignes, 97,5 % des branchements. **Arrivée : 100 %** des lignes,
  branchements, fonctions et régions.
- **Traitement, un commit justifié par portion** :

| Portion non couverte | Diagnostic | Traitement |
| --- | --- | --- |
| `SystemClock::today` | Situation non testée | Test ajouté |
| `open()` en échec et en succès | Situation non testée | Port inexistant ; pseudo-terminal sous Linux |
| Décimales non numériques (`145.5a`) | Branchement jamais pris | Test ajouté |
| Partie entière trop grande pour un `u64` | Situation non testée | Test ajouté |
| Gestion d'erreur du calcul des centimes | **Code superflu** : erreur impossible | Code simplifié |
| Hello world (`main.rs`, `greeting()`) | **Code superflu** | Remplacé par une vraie CLI |

`src/main.rs` est exclu de la mesure : c'est du code d'assemblage sans logique métier, testé
à part en lançant le programme (étape 6).

## Étape 6 — Tests de mutation (#12)

L'outil `cargo-mutants` modifie automatiquement le code (`>` devient `>=`, `&&` devient `||`, une
fonction renvoie une valeur par défaut…). Si les tests passent encore, le mutant **survit** :
les tests ne détectent pas cette erreur.

- **Campagne initiale** : 63 mutants, dont **4 survivants**, chacun suivi par une issue.

| Issue | Mutant survivant | Renforcement |
| --- | --- | --- |
| #11 | `&&` → `\|\|` dans le contrôle du format de date | Cas `2026-11-1` (seule la longueur est fausse) |
| #9, #10 | `>` → `==` ou `>=` sur la limite de 256 octets | Test à la valeur limite exacte |
| #8 | `main()` remplacé par « ne rien faire » | Tests qui lancent la CLI et vérifient ses codes de sortie |

- Deux mutants provoquaient un **timeout** (boucle infinie) au lieu d'un échec : le faux port a
  été ajusté pour qu'ils échouent immédiatement.
- **Campagne finale : 0 mutant survivant** (60 tués, 3 qui ne compilent pas).

## Qualité du projet

Chaque pull request est vérifiée automatiquement par l'intégration continue :
formatage, analyse statique (`clippy`), tests sur Linux, Windows et macOS, version minimale de
Rust, documentation, **couverture (seuil 100 % des lignes)** et **tests de mutation
(aucun survivant toléré)**.

## Résultats

| Indicateur | Valeur |
| --- | --- |
| Tests | 60 (55 du module, 1 de l'horloge, 3 de la CLI, 1 exemple de documentation) |
| Couverture lignes / branchements | 100 % / 100 % |
| Mutants survivants | 0 sur 63 |
| Défauts du code généré trouvés par les tests | 6 cas, corrigés en 3 commits |

Les commandes pour reproduire ces mesures sont dans le [README](../README.md#développement).

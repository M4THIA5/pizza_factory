# 🍕 Pizza Factory

Système distribué de « production de pizzas » en Rust : chaque nœud expose des **capacités** (actions), découvre les autres via **gossip UDP** (CBOR), et sert des **commandes TCP** (CBOR encadré) compatibles avec le protocole attendu par `pizza_factory`.

## Prérequis

- Rust avec édition **2024** (toolchain récente, `cargo` à jour)

## Compilation

```bash
cargo build
```

Contrôle strict des avertissements :

```bash
cargo clippy -- -D warnings
```

## Arborescence

```text
src/
├── main.rs           # Point d’entrée : `server` | `client`
├── cli.rs            # Arguments (clap)
├── protocol.rs       # Messages UDP gossip : GossipMessage, GossipPayload, Version
├── recipe.rs         # Chargement et parsing des recettes (DSL)
├── wire/             # Trames TCP communes : longueur u32 BE + CBOR
│   ├── mod.rs
│   └── framing.rs
├── client/           # Client TCP (order, list-recipes, get-recipe)
│   └── mod.rs
└── server/
    ├── mod.rs        # GossipState, run_server()
    ├── udp.rs        # Écoute + émission gossip (Ping / Pong / Announce)
    └── tcp.rs        # Serveur TCP CBOR + REPL
```

## Serveur

### Rôle

- **UDP** : annonces d’état (`Announce`), échanges `Ping` / `Pong` pour maintenir la vue des pairs (capacités, recettes annoncées, version).
- **TCP** : même adresse `host:port` ; protocole par **trames** : 4 octets (longueur big-endian du corps) + corps **CBOR**.
- **REPL** (stdin) : commandes `peers`, `recipes`, `capabilities`, `quit`.

### Exemple — trois nœuds

Chaque instance écoute sur un port différent. Les `--peer` amorcent la table ; le gossip propage la topologie.

```bash
# Terminal 1 — nœud A (8000)
cargo run -- server --addr 127.0.0.1:8000 --capabilities MakeDough,Bake

# Terminal 2 — nœud B (8001), pair A
cargo run -- server \
  --addr 127.0.0.1:8001 \
  --capabilities AddCheese,AddBasil \
  --gossip-interval 5 \
  --peer 127.0.0.1:8000

# Terminal 3 — nœud C (8002), pair A
cargo run -- server \
  --addr 127.0.0.1:8002 \
  --capabilities AddOliveOil,AddPepperoni \
  --recipes assets/pizza.recipes \
  --peer 127.0.0.1:8000
```

### Options principales

| Option | Description |
| --- | --- |
| `--addr` | Adresse d’écoute UDP + TCP (défaut : `127.0.0.1:8000`) |
| `--peer` | Pair bootstrap, répétable |
| `--capabilities` | Liste d’actions supportées (répétable ou une fois avec des virgules selon le shell) |
| `--recipes` | Fichier de recettes (défaut : `assets/pizza.recipes`) |
| `--gossip-interval` | Intervalle en secondes entre les cycles de gossip (défaut : `10`) |

Aide : `cargo run -- server --help`

### REPL

| Commande | Effet |
| --- | --- |
| `peers` | Pairs connus : version, capacités, recettes vues dans les annonces |
| `recipes` | Recettes chargées localement depuis le fichier |
| `capabilities` | Capacités déclarées par ce nœud |
| `quit` / `exit` | Quitte le processus |

## Client

Même encodage TCP que `pizza_factory client` (trames longueur + CBOR).

```bash
# Passer une commande (recette) sur un nœud
cargo run -- client --peer 127.0.0.1:8000 order Minimal

# Lister les recettes exposées par le serveur
cargo run -- client --peer 127.0.0.1:8000 list-recipes

# Obtenir le détail d’une recette
cargo run -- client --peer 127.0.0.1:8000 get-recipe Margherita
```

Aide : `cargo run -- client --help`

## Protocoles

### UDP — Gossip

Messages **CBOR** en map à une clé typée, par exemple :

- `Ping` / `Pong` : charge utile avec `version` (`counter`, `generation`) et métadonnées (`last_seen`, etc.).
- `Announce` : `node_addr`, `capabilities`, `recipes`, `peers`, `version`.

À la réception d’un `Announce` décrivant un **nouveau** pair, le nœud peut incrémenter sa version et **rediffuser** un `Announce` pour rester cohérent avec les réponses `Pong`.

### TCP — Commandes

1. Le client envoie une trame : longueur `u32` big-endian, puis une map CBOR dont la première clé indique la commande : `order`, `list_recipes`, `get_recipe`.
2. Pour `order`, le serveur répond notamment par `order_receipt` (identifiant de commande, tag CBOR **37**), puis `completed_order` dont le champ `result` peut contenir une chaîne JSON décrivant contenu et mises à jour (actions locales, éventuel `Forward` vers un autre pair tag **260**, `Deliver`, etc.).

Les journaux préfixés `[TCP]` sur le serveur détaillent les actions locales et les transferts vers un pair lorsque la capacité n’est pas disponible sur le nœud courant.

## Format des recettes

Fichier texte, blocs séparés par une ligne vide :

```text
Margherita =
    MakeDough
    -> AddBase(base_type=tomato)
    -> [AddCheese(amount=2), AddBasil(leaves=3)]
    -> Bake(duration=5)
    -> AddOliveOil
```

- `->` : enchaînement d’étapes.
- `[A, B]` : étapes en parallèle.
- `Action(p=v,…)` : paramètres nommés.
- `Action^n` : répétition (syntaxe du parser).

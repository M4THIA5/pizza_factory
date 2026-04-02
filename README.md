# 🍕 Pizza Agent

Système distribué de production de pizzas en Rust. Chaque agent connaît un sous-ensemble de capacités et collabore avec les autres via un protocole de découverte UDP (gossip) et des échanges TCP.

## Architecture

```text
src/
├── main.rs           # Point d'entrée, dispatch des sous-commandes
├── cli.rs            # Arguments CLI (clap)
├── protocol.rs       # Types partagés : GossipMessage, Version, GossipPayload
├── recipe.rs         # Parser de recettes (DSL)
└── server/
    ├── mod.rs        # Commun à TCP et UDP : GossipState, run_server()
    ├── udp.rs        # Gossip : start_udp_listener(), start_gossip_emitter()
    └── tcp.rs        # Production : start_tcp_server(), handle_tcp_connection()
└── client/
```

## Prérequis

- Rust 2021 (cargo 1.70+)

## Compilation

```bash
cargo build
```

## Lancer 3 serveurs

Chaque serveur tourne sur un port différent. Le premier n'a pas de pair connu — les autres lui passent son adresse au démarrage. La découverte se propage ensuite automatiquement via le gossip UDP.

```bash
# Terminal 1 — Agent A (port 8000, pas de pair connu)
cargo run -- server \
  --capabilities MakeDough,Bake

# Terminal 2 — Agent B (port 8001, connaît A)
cargo run -- server \
  --addr 127.0.0.1:8001 \
  --capabilities AddCheese,AddBasil \
  --peer 127.0.0.1:8000

# Terminal 3 — Agent C (port 8002, connaît A)
cargo run -- server \
  --addr 127.0.0.1:8002 \
  --capabilities AddOliveOil,AddPepperoni \
  --recipes assets/pizza.recipes \
  --peer 127.0.0.1:8001
```

Au bout de 10 secondes (premier cycle gossip), tous les agents se connaissent mutuellement.

## Commandes interactives (REPL)

Une fois un serveur lancé, un REPL est disponible dans le terminal :

```text
peers         — liste les pairs connus et leur version
recipes       — affiche les recettes chargées
capabilities  — affiche les capacités de cet agent
quit          — arrête le serveur
```

## Protocole

### UDP — Gossip (découverte)

Chaque agent envoie un `Ping` UDP à tous ses pairs connus toutes les **10 secondes**. À la réception d'un `Ping`, l'agent enregistre l'expéditeur et répond avec un `Pong`.

Les messages sont sérialisés en **CBOR** sous la forme d'un tableau à 2 éléments :

```text
["Ping", { last_seen: u64, version: { counter: u64, generation: u64 } }]
["Pong", { last_seen: u64, version: { counter: u64, generation: u64 } }]
```

- `last_seen` : timestamp en millisecondes
- `counter` : incrémenté à chaque message émis
- `generation` : timestamp de démarrage du nœud (permet de détecter un redémarrage)

### TCP — Production

Les commandes de production (à venir) transitent en TCP pour garantir la fiabilité de la transmission.

## Format des recettes

Les recettes sont décrites dans un fichier texte avec la syntaxe suivante :

```text
Margherita =
    MakeDough
    -> AddBase(base_type=tomato)
    -> [AddCheese(amount=2), AddBasil(leaves=3)]
    -> Bake(duration=5)
    -> AddOliveOil
```

- `->` : séquence d'étapes
- `[A, B]` : étapes parallèles (ordre libre)
- `Action(param=value)` : action avec paramètres
- `Action^n` : répétition d'une action n fois

## Arguments disponibles

```text
Usage: pizza-agent server [OPTIONS]

Options:
  -a, --addr <ADDR>              Adresse d'écoute [défaut: 127.0.0.1:8001]
  -p, --peer <PEER>              Pair connu au démarrage (répétable)
  -c, --capabilities <CAP>       Capacités de cet agent (séparées par virgule)
  -r, --recipes <PATH>           Fichier de recettes [défaut: assets/pizza.recipes]
  -h, --help                     Affiche l'aide
```

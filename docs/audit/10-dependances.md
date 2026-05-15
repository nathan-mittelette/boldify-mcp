# Audit — Dépendances : versions et optimisations

**Sévérité**: 🟡 Moyen  
**Fichiers**: `Cargo.toml` (workspace), `converter/Cargo.toml`, `mcp/Cargo.toml`

---

## Problème 1 — `tokio/full` dans la feature `http`

```toml
# mcp/Cargo.toml
[features]
http = ["dep:axum", "tokio/full"]
```

La feature `tokio/full` active **toutes** les fonctionnalités de tokio : net, io, time, signal, process, fs, sync… La majorité sont inutilisées par un serveur HTTP Axum basique.

**Recommandation** : Activer uniquement les features nécessaires :

```toml
[features]
http = [
    "dep:axum",
    "tokio/net",
    "tokio/io-util",
    "tokio/macros",
    "tokio/rt-multi-thread",
]
```

**Impact** : Réduction du temps de compilation et de la taille du binaire.

---

## Problème 2 — `once_cell` (voir audit #06)

```toml
# converter/Cargo.toml
once_cell = "1"
```

Remplacer par `std::sync::OnceLock` (disponible depuis Rust 1.80). Supprime une dépendance externe.

---

## Problème 3 — `anyhow` utilisé minimalement dans `mcp`

```toml
# mcp/Cargo.toml
anyhow = "1"
```

`anyhow` est importé uniquement pour le type `Result<()>` dans `main.rs`. Rust 2021 permet d'utiliser `Box<dyn std::error::Error>` ou une simple conversion. Si le seul usage est `fn main() -> anyhow::Result<()>`, l'overhead de la dépendance est disproportionné.

**Recommandation** : Évaluer si `anyhow` apporte assez de valeur. Alternatives :

```rust
// Sans anyhow
fn main() -> Result<(), Box<dyn std::error::Error>> { ... }
```

Ou si `anyhow` est utilisé plus largement, le conserver et documenter pourquoi.

---

## Problème 4 — Pas de version minimale de Rust documentée

Il n'existe pas de `rust-toolchain.toml` ni de champ `rust-version` dans `Cargo.toml`. La CI utilise probablement `stable` implicitement, mais cela n'est pas vérifiable sans lire le workflow.

**Recommandation** : Ajouter dans `Cargo.toml` :

```toml
[workspace]
resolver = "2"
rust-version = "1.80"  # OnceLock stable depuis 1.80
```

Et/ou un `rust-toolchain.toml` :

```toml
[toolchain]
channel = "stable"
```

---

## Tableau récapitulatif

| Dépendance | Crate | Statut | Action |
|-----------|-------|--------|--------|
| `once_cell` | `converter` | Obsolète (remplacé par stdlib) | Supprimer, migrer vers `OnceLock` |
| `tokio/full` | `mcp` (feature http) | Surdimensionné | Remplacer par features ciblées |
| `anyhow` | `mcp` | Usage minimal | Évaluer suppression |
| `rust-version` | workspace | Absent | Ajouter `rust-version = "1.80"` |

---

## Dépendances sans problème (pour info)

| Dépendance | Version | Commentaire |
|-----------|---------|------------|
| `thiserror` | `2` | Récent, stable |
| `unicode-segmentation` | `1` | Seule option viable pour les graphèmes |
| `rmcp` | `1.5` | Version spécifique, surveiller les mises à jour |
| `axum` | `0.8` | Version actuelle |
| `serde` | `1` | Standard |
| `lambda_http` | `0.14` | Vérifier compatibilité avec runtime Lambda |

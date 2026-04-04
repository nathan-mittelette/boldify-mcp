# Tâche 00 — Initialisation du workspace Rust

## Scope

Créer la structure de base du workspace Cargo multi-crates. Aucune logique métier dans cette tâche — uniquement les fichiers `Cargo.toml`, les répertoires `src/` vides avec un `lib.rs` / `main.rs` minimal, et les dépendances déclarées.

**Référence** : [`01-architecture.md`](../01-architecture.md)

---

## Structure à créer

```
boldify-mcp/
├── Cargo.toml              ← workspace root
├── parser/
│   ├── Cargo.toml
│   └── src/lib.rs
├── converter/
│   ├── Cargo.toml
│   └── src/lib.rs
├── service/
│   ├── Cargo.toml
│   └── src/lib.rs
├── api-syntaxes/
│   ├── Cargo.toml
│   └── src/main.rs
├── api-convert/
│   ├── Cargo.toml
│   └── src/main.rs
└── mcp/
    ├── Cargo.toml
    └── src/main.rs
```

---

## Workspace `Cargo.toml`

```toml
[workspace]
members = [
    "parser",
    "converter",
    "service",
    "api-syntaxes",
    "api-convert",
    "mcp",
]
resolver = "2"
```

---

## Dépendances par crate

### `parser/Cargo.toml`
```toml
[package]
name    = "parser"
version = "0.1.0"
edition = "2021"

[dependencies]
thiserror = "1"
serde     = { version = "1", features = ["derive"] }
```

### `converter/Cargo.toml`
```toml
[package]
name    = "converter"
version = "0.1.0"
edition = "2021"

[dependencies]
parser               = { path = "../parser" }
unicode-segmentation = "1"
once_cell            = "1"
```

### `service/Cargo.toml`
```toml
[package]
name    = "service"
version = "0.1.0"
edition = "2021"

[dependencies]
parser    = { path = "../parser" }
converter = { path = "../converter" }
thiserror = "1"
```

### `api-syntaxes/Cargo.toml`
```toml
[package]
name    = "api-syntaxes"
version = "0.1.0"
edition = "2021"

[dependencies]
service      = { path = "../service" }
lambda_http  = "0.12"
serde_json   = "1"
tokio        = { version = "1", features = ["full"] }
```

### `api-convert/Cargo.toml`
```toml
[package]
name    = "api-convert"
version = "0.1.0"
edition = "2021"

[dependencies]
service      = { path = "../service" }
lambda_http  = "0.12"
serde_json   = "1"
tokio        = { version = "1", features = ["full"] }
```

### `mcp/Cargo.toml`
```toml
[package]
name    = "mcp"
version = "0.1.0"
edition = "2021"

[features]
default = []
cli     = ["rmcp/transport-io"]
http    = ["rmcp/transport-sse-server", "tokio/full", "dep:axum"]

[dependencies]
service    = { path = "../service" }
rmcp       = { version = "0.1" }
serde_json = "1"
tokio      = { version = "1", optional = true }
axum       = { version = "0.7", optional = true }
```

---

## Contenu minimal des `src/lib.rs` / `src/main.rs`

Chaque `lib.rs` contient uniquement :
```rust
// placeholder
```

Chaque `main.rs` contient :
```rust
fn main() {}
```

---

## Critère de succès

```bash
cargo check --workspace
```

Doit passer sans erreur. Aucun test à écrire pour cette tâche.

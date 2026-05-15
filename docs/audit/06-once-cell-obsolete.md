# Audit — Dépendance `once_cell` obsolète

**Sévérité**: 🟠 Haute  
**Crates concernées**: `converter`  
**Fichiers**: `converter/src/handlers/bold.rs`, `italic.rs`, `strikethrough.rs`, `underline.rs`, `surline.rs`, `Cargo.toml`

---

## Problème

La crate `once_cell` est utilisée pour initialiser les `HashMap` de correspondance une seule fois :

```rust
// converter/src/handlers/bold.rs
use once_cell::sync::OnceCell;

static BOLD_MAP: OnceCell<HashMap<String, String>> = OnceCell::new();

fn bold_map() -> &'static HashMap<String, String> {
    BOLD_MAP.get_or_init(|| { ... })
}
```

Depuis Rust 1.80 (stabilisé en juillet 2024), la bibliothèque standard expose `std::sync::OnceLock` qui remplace exactement ce cas d'usage. La crate `once_cell` est elle-même en mode maintenance et recommande la migration vers `std`.

---

## Recommandation

### 1. Supprimer la dépendance dans `converter/Cargo.toml`

```toml
# Supprimer cette ligne
once_cell = "1"
```

### 2. Remplacer l'import dans chaque handler

```rust
// Avant
use once_cell::sync::OnceCell;
static BOLD_MAP: OnceCell<HashMap<String, String>> = OnceCell::new();

// Après
use std::sync::OnceLock;
static BOLD_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
```

L'API est identique : `OnceLock::new()` et `.get_or_init(|| { ... })` fonctionnent exactement comme leurs équivalents `once_cell`.

### 3. Fichiers à modifier

| Fichier | Import à changer | Static à changer |
|---------|-----------------|-----------------|
| `handlers/bold.rs` | `OnceCell` → `OnceLock` | `OnceCell::new()` → `OnceLock::new()` |
| `handlers/italic.rs` | idem | idem |
| `handlers/strikethrough.rs` | idem | idem |
| `handlers/underline.rs` | idem | idem |
| `handlers/surline.rs` | idem | idem |

---

## Vérification de la version Rust

Avant de migrer, vérifier que le `rust-toolchain.toml` ou `Cargo.toml` cible bien Rust 1.80+ :

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"  # stable >= 1.80 au moment de l'audit (2026-05)
```

`OnceLock` est disponible depuis Rust 1.80.0 — toute toolchain stable récente convient.

---

## Impact attendu

- Suppression d'une dépendance externe
- Compilation légèrement plus rapide (une crate de moins)
- Alignement avec les idiomes Rust modernes
- Pas de changement de comportement (API identique)

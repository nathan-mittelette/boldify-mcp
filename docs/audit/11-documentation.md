# Audit — Documentation manquante

**Sévérité**: 🟢 Faible  
**Crates concernées**: `parser`, `converter`, `service`, `mcp`

---

## Problème

Les modules et fonctions publics n'ont pas de documentation Rust (`///` ou `//!`). Le code est lisible mais un contributeur nouveau ou externe ne peut pas comprendre les invariants, les cas limites et les choix de conception sans lire le code en entier.

---

## Modules sans doc

| Fichier | Ce qui manque |
|---------|--------------|
| `parser/src/lib.rs` | Doc du module : quels formats acceptés, quel AST produit |
| `converter/src/lib.rs` | Doc du module : transformation AST → Unicode |
| `converter/src/traits.rs` | Doc du trait `ToUnicode` : contrat, invariants |
| `service/src/lib.rs` | Doc de `ContentService` : point d'entrée public |
| `mcp/src/server.rs` | Doc des outils MCP exposés |

---

## Exemples de documentation à ajouter

### Trait `ToUnicode`

```rust
/// Converts an AST node to Unicode-formatted text.
///
/// Implementors must handle all grapheme clusters, including:
/// - ASCII letters and digits (transformed via Unicode font variants)
/// - Accented characters (transformed via the accent map)
/// - Spaces and newlines (passed through unchanged)
/// - Emoji and other non-transformable graphemes (passed through unchanged)
pub trait ToUnicode {
    fn to_unicode(&self) -> String;
}
```

### `ContentService`

```rust
/// Orchestrates parsing and Unicode conversion.
///
/// Accepts Markdown or HTML input, parses it into an AST, then converts
/// each node to Unicode-formatted text using the configured style handlers.
///
/// # Example
/// ```
/// let svc = ContentService::new();
/// let result = svc.convert("**bold**", "markdown").unwrap();
/// assert_eq!(result, "𝐛𝐨𝐥𝐝");
/// ```
pub struct ContentService { ... }
```

### Module `parser`

```rust
//! Parses Markdown and HTML input into an Abstract Syntax Tree (AST).
//!
//! # Supported Markdown
//! - `**bold**` / `__bold__`
//! - `*italic*` / `_italic_`
//! - `~~strikethrough~~`
//! - `==highlight==`
//! - Unordered lists (`-`) and ordered lists (`1.`)
//!
//! # Supported HTML tags
//! `<b>`, `<strong>`, `<i>`, `<em>`, `<u>`, `<mark>`, `<s>`, `<del>`,
//! `<ul>`, `<ol>`, `<li>`, `<p>`, `<br>`
//!
//! # Errors
//! Returns [`ParseError`] for unsupported constructs (headings, code blocks,
//! tables) or malformed input (unclosed tags, exceeded size limit).
```

---

## Doctests

Les doctests servent à la fois de documentation et de tests de régression. Exemple :

```rust
/// # Examples
/// ```
/// use parser::MarkdownParser;
/// use parser::Parser;
///
/// let nodes = MarkdownParser.parse("**hello**").unwrap();
/// assert_eq!(nodes.len(), 1);
/// ```
```

---

## Priorité de documentation

| Priorité | Cible |
|----------|-------|
| 1 (haute) | `ContentService` — point d'entrée public |
| 2 | Trait `ToUnicode` et trait `Parser` |
| 3 | `ParseError` variants (chaque variante) |
| 4 | Modules `parser`, `converter`, `service` |
| 5 (faible) | Fonctions internes `parse_inline`, `flush_text`, etc. |

---

## Impact attendu

- `cargo doc` génère une documentation navigable
- Les contributeurs comprennent les invariants sans lire tout le code
- Les doctests détectent les régressions d'interface publique

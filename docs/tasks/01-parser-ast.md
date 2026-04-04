# Tâche 01 — Définition de l'AST du parser

## Scope

Implémenter tous les types de l'AST dans `parser/src/ast.rs` ainsi que les erreurs dans `parser/src/error.rs`. Cette tâche ne touche **aucune autre couche**. Aucun parser fonctionnel n'est implémenté ici — uniquement les types.

**Référence** : [`02-ast-nodes.md`](../02-ast-nodes.md), [`03-parser.md`](../03-parser.md)

---

## Fichiers à créer

```
parser/src/
├── lib.rs       ← re-exports publics + trait Parser + SupportedSymbol
├── ast.rs       ← tous les types AST
└── error.rs     ← ParseError + SourcePosition
```

---

## `parser/src/ast.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self { Self { start, end } }
    pub fn len(&self) -> usize { self.end.saturating_sub(self.start) }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeBase {
    pub id: u64,
    pub span: Span,
}

impl NodeBase {
    pub fn new(id: u64, span: Span) -> Self { Self { id, span } }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerType {
    Text,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Surline,
    Blockquote,
    List,
    OrderedList,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    pub base: NodeBase,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItemNode {
    pub base: NodeBase,
    pub children: Vec<InlineNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineNode {
    Text(TextNode),
    Container(ContainerNode),
    ListItem(ListItemNode),
}

impl InlineNode {
    pub fn base(&self) -> &NodeBase {
        match self {
            InlineNode::Text(n)      => &n.base,
            InlineNode::Container(n) => &n.base,
            InlineNode::ListItem(n)  => &n.base,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerNode {
    pub base: NodeBase,
    pub container_type: ContainerType,
    pub children: Vec<InlineNode>,
}
```

---

## `parser/src/error.rs`

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ligne {}, colonne {} (offset {})", self.line, self.column, self.byte_offset)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(
        "Symbole non supporté : `{symbol}` à la position {position}.\n\
         Consultez la liste des syntaxes supportées via :\n\
         - API HTTP : GET /syntaxes\n\
         - MCP CLI  : mcp list"
    )]
    UnsupportedSymbol {
        symbol: String,
        position: SourcePosition,
    },

    #[error("Document HTML invalide : {0}")]
    InvalidHtml(String),

    #[error("Balise HTML non fermée : `{tag}` ouverte à {position}")]
    UnclosedTag {
        tag: String,
        position: SourcePosition,
    },
}
```

---

## `parser/src/lib.rs`

```rust
pub mod ast;
pub mod error;

pub use ast::*;
pub use error::ParseError;

pub trait Parser {
    fn parse(&self, input: &str) -> Result<Vec<ContainerNode>, ParseError>;
    fn supported_symbols(&self) -> Vec<SupportedSymbol>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SupportedSymbol {
    pub symbol: String,
    pub description: String,
    pub example: String,
}
```

---

## Tests à implémenter

Fichier : `parser/src/ast.rs` (module `#[cfg(test)]` en fin de fichier)

### Span

```rust
#[test]
fn span_len_calcule_correctement() {
    let s = Span::new(3, 10);
    assert_eq!(s.len(), 7);
}

#[test]
fn span_vide_detecte() {
    let s = Span::new(5, 5);
    assert!(s.is_empty());
}

#[test]
fn span_len_sature_si_end_inferieur_a_start() {
    // saturating_sub : pas de panique si end < start
    let s = Span::new(10, 5);
    assert_eq!(s.len(), 0);
    assert!(s.is_empty());
}
```

### ContainerNode

```rust
#[test]
fn container_node_children_vide_par_defaut() {
    let node = ContainerNode {
        base: NodeBase::new(0, Span::new(0, 0)),
        container_type: ContainerType::Text,
        children: vec![],
    };
    assert!(node.children.is_empty());
}

#[test]
fn inline_node_base_retourne_bonne_reference() {
    let text_node = TextNode {
        base: NodeBase::new(42, Span::new(1, 5)),
        text: "hello".to_string(),
    };
    let inline = InlineNode::Text(text_node);
    assert_eq!(inline.base().id, 42);
    assert_eq!(inline.base().span.start, 1);
}
```

### ParseError

```rust
#[test]
fn parse_error_unsupported_affiche_le_symbole() {
    let err = ParseError::UnsupportedSymbol {
        symbol: "<div>".to_string(),
        position: SourcePosition { line: 3, column: 7, byte_offset: 42 },
    };
    let msg = err.to_string();
    assert!(msg.contains("<div>"));
    assert!(msg.contains("GET /syntaxes"));
}

#[test]
fn source_position_affiche_ligne_et_colonne() {
    let pos = SourcePosition { line: 2, column: 5, byte_offset: 20 };
    assert!(pos.to_string().contains("ligne 2"));
    assert!(pos.to_string().contains("colonne 5"));
}
```

---

## Critère de succès

```bash
cargo test --package parser
```

Tous les tests passent. `cargo check --workspace` reste vert.

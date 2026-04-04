# AST : Nœuds et Types du Parser

## Principes de conception

L'AST produit par le parser est défini entièrement dans le crate `parser`. Deux règles fondamentales s'appliquent :

1. **Uniformité** : tout le document est représenté par des `ContainerNode`. Un `ContainerNode` porte son type et contient une liste d'`InlineNode`. Pas de `String` brute à la racine d'un nœud.
2. **Isolation** : le parser ne connaît ni le trait `ToUnicode`, ni aucun type défini dans `converter`.

---

## Types de conteneurs supportés

### Markdown

| Symbole | Type produit |
|---|---|
| texte nu | `ContainerType::Text` |
| `**…**` | `ContainerType::Bold` |
| `*…*` ou `_…_` | `ContainerType::Italic` |
| `~~…~~` | `ContainerType::Strikethrough` |
| `==…==` | `ContainerType::Surline` |
| `> …` | `ContainerType::Blockquote` |
| `- …` / `* …` | `ContainerType::List` |
| `1. …` | `ContainerType::OrderedList` |
| `\n` | saut de ligne dans le `TextNode` courant |

> **Note** : `#` (titres Markdown) n'est **pas** supporté. Il n'existe pas d'équivalent Unicode pour la notion de titre hiérarchique — le parser retourne `ParseError::UnsupportedSymbol` si `#` est rencontré.

Tout autre symbole → `ParseError::UnsupportedSymbol`.

### HTML

Le HTML supporte tout ce que Markdown supporte, plus :

| Tag | Type produit |
|---|---|
| `<strong>`, `<b>` | `ContainerType::Bold` |
| `<em>`, `<i>` | `ContainerType::Italic` |
| `<u>` | `ContainerType::Underline` |
| `<mark>` | `ContainerType::Surline` |
| `<s>`, `<del>` | `ContainerType::Strikethrough` |
| `<blockquote>` | `ContainerType::Blockquote` |
| `<ul>` | `ContainerType::List` |
| `<ol>` | `ContainerType::OrderedList` |
| `<li>` | `ContainerType::ListItem` |
| `<br>` | saut de ligne dans le `TextNode` courant |
| `<p>` | ignoré à l'ouverture |
| `</p>` | insère un `\n` dans le flux courant |

> **Note** : `<h1>`–`<h6>` ne sont **pas** supportés pour la même raison que `#` en Markdown — pas d'équivalent Unicode pour les niveaux de titre. Tout autre tag (`<div>`, `<span>`, `<table>`, etc.) génère une erreur immédiate.

---

## `ContainerType`

```rust
// parser/src/ast.rs

/// Nature sémantique d'un `ContainerNode`.
///
/// Pas de variant `Heading` : il n'existe pas d'équivalent Unicode
/// pour les niveaux de titre (h1–h6, #–######). Le parser rejette
/// ces symboles avec `ParseError::UnsupportedSymbol`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerType {
    /// Texte ordinaire (pas de symbole déclencheur).
    Text,
    /// `**…**` (Markdown) / `<strong>`, `<b>` (HTML).
    Bold,
    /// `*…*`, `_…_` (Markdown) / `<em>`, `<i>` (HTML).
    Italic,
    /// `<u>` (HTML uniquement — pas de syntaxe Markdown native).
    Underline,
    /// `~~…~~` (Markdown) / `<s>`, `<del>` (HTML).
    Strikethrough,
    /// `==…==` (Markdown étendu) / `<mark>` (HTML).
    Surline,
    /// `> …` (Markdown) / `<blockquote>` (HTML).
    Blockquote,
    /// `- …` / `* …` (Markdown) / `<ul>` (HTML).
    /// Les enfants sont exclusivement des `InlineNode::ListItem`.
    List,
    /// `1. …` (Markdown) / `<ol>` (HTML).
    /// Les enfants sont exclusivement des `InlineNode::ListItem`.
    OrderedList,
}
```

---

## Structure des nœuds

### `NodeBase`

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
```

### `TextNode` — feuille de texte

```rust
/// Fragment de texte atomique. Toujours une feuille (pas d'enfants).
/// Le texte peut contenir des `\n` (issus de `<br>`, `</p>` ou sauts de ligne Markdown).
#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    pub base: NodeBase,
    pub text: String,
}
```

> **Note** : le style du `TextNode` est porté par le `ContainerType` de son parent direct. Un `TextNode` n'a pas de style propre — c'est son `ContainerNode` parent qui le définit.

### `ListItemNode` — item de liste

```rust
/// Item d'une liste. Contient des fragments inline.
/// Uniquement présent comme `InlineNode::ListItem` enfant d'un
/// `ContainerNode { type: List | OrderedList }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItemNode {
    pub base: NodeBase,
    pub children: Vec<InlineNode>,
}
```

### `InlineNode` — nœud inline

```rust
/// Contenu d'un `ContainerNode`.
#[derive(Debug, Clone, PartialEq)]
pub enum InlineNode {
    /// Fragment de texte (feuille).
    Text(TextNode),
    /// Sous-conteneur stylisé (Bold, Italic, etc.).
    Container(ContainerNode),
    /// Item de liste. Uniquement valide dans un `ContainerNode { type: List | OrderedList }`.
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
```

### `ContainerNode` — nœud universel

```rust
/// Nœud conteneur universel.
///
/// Au niveau document : paragraphe, titre, liste, citation, etc.
/// Au niveau inline   : sous-groupe stylisé (Bold contenant de l'Italic, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerNode {
    pub base: NodeBase,
    pub container_type: ContainerType,
    pub children: Vec<InlineNode>,
}
```

---

## Fichier complet `parser/src/ast.rs`

```rust
// parser/src/ast.rs

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
    Heading(u8),
    Blockquote,
    List,
    OrderedList,
}

/// Fragment de texte atomique (feuille). Peut contenir des `\n`.
#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    pub base: NodeBase,
    pub text: String,
}

/// Item d'une liste.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItemNode {
    pub base: NodeBase,
    pub children: Vec<InlineNode>,
}

/// Nœud inline : feuille, sous-conteneur stylisé, ou item de liste.
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

/// Nœud conteneur universel.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerNode {
    pub base: NodeBase,
    pub container_type: ContainerType,
    pub children: Vec<InlineNode>,
}
```

> Le parser retourne `Vec<ContainerNode>`. Le style d'un `TextNode` est implicitement celui du `ContainerType` de son parent le plus proche.

---

## Ce que le parser ne fait PAS

- Il ne définit pas le trait `ToUnicode`.
- Il ne connaît pas le crate `converter`.
- Il ne tolère aucun symbole ou tag non listé dans les tableaux ci-dessus.

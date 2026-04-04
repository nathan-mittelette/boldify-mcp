# Couche Parser

## Responsabilité

Le crate `parser` transforme du texte brut (HTML ou Markdown) en `Vec<ContainerNode>`. Il est **strict** : tout symbole ou tag non reconnu provoque une erreur immédiate. Il ne connaît ni le converter, ni le service.

---

## Trait `Parser`

```rust
// parser/src/lib.rs

pub mod ast;
pub mod error;
pub mod html;
pub mod id;
pub mod inline;
pub mod markdown;

pub use ast::*;
pub use error::ParseError;

pub trait Parser {
    /// Parse `input` et retourne une liste ordonnée de `ContainerNode`.
    ///
    /// Un contenu vide produit `Ok(vec![])`.
    /// Tout symbole ou tag non supporté produit `Err(ParseError::UnsupportedSymbol)`.
    fn parse(&self, input: &str) -> Result<Vec<ContainerNode>, ParseError>;

    /// Retourne la liste des symboles/tags supportés par ce parser.
    ///
    /// Appelé par le service pour répondre aux requêtes `list_syntaxes`.
    /// C'est le parser lui-même qui est source de vérité sur ce qu'il accepte —
    /// pas le service, pas l'API.
    fn supported_symbols(&self) -> Vec<SupportedSymbol>;
}

/// Descripton d'un symbole supporté par un parser.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SupportedSymbol {
    /// Représentation du symbole tel qu'il apparaît dans la source.
    /// Ex: `"**"`, `"*"`, `"~~"`, `"<strong>"`.
    pub symbol: String,
    /// Description humaine de l'effet produit.
    /// Ex: `"Gras"`, `"Italique"`, `"Texte barré"`.
    pub description: String,
    /// Exemple d'utilisation dans la syntaxe du parser.
    /// Ex: `"**texte gras**"`, `"<strong>texte</strong>"`.
    pub example: String,
}
```

---

## `ParseError`

```rust
// parser/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// Symbole ou tag rencontré mais non supporté par ce parser.
    ///
    /// `symbol`   : le symbole ou tag exact (ex: "`", "<div>", "<span>").
    /// `position` : position dans le texte source (ligne, colonne).
    /// Le message guide l'utilisateur vers la liste des syntaxes supportées.
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

    #[error("Entrée trop volumineuse : {found} octets (maximum : {max})")]
    InputTooLarge { found: usize, max: usize },
}

/// Position humainement lisible dans le texte source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePosition {
    /// Numéro de ligne (commence à 1).
    pub line: usize,
    /// Numéro de colonne en caractères (commence à 1).
    pub column: usize,
    /// Indice d'octet absolu dans la chaîne source.
    pub byte_offset: usize,
}

impl std::fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ligne {}, colonne {}", self.line, self.column)
    }
}
```

### Exemple de message d'erreur

```
Symbole non supporté : `<div>` à la position ligne 4, colonne 1.
Consultez la liste des syntaxes supportées via :
- API HTTP : GET /syntaxes
- MCP CLI  : mcp list
```

```
Symbole non supporté : `<span>` à la position ligne 2, colonne 15.
Consultez la liste des syntaxes supportées via :
- API HTTP : GET /syntaxes
- MCP CLI  : mcp list
```

---

## `IdGenerator`

```rust
// parser/src/id.rs

pub struct IdGenerator(u64);

impl IdGenerator {
    pub fn new() -> Self { Self(0) }
    pub fn next(&mut self) -> u64 {
        let id = self.0;
        self.0 += 1;
        id
    }
}
```

---

> Les algorithmes détaillés de chaque parser sont documentés dans des fichiers dédiés :
> - **`03a-parser-markdown.md`** — flux bloc par bloc, parsing inline caractère par caractère, exemples pas à pas
> - **`03b-parser-html.md`** — validation des tags, traversée du DOM, gestion du whitespace

---

## Parsing inline (Markdown)

La fonction `parse_inline` segmente une chaîne en `Vec<InlineNode>`. Elle s'arrête immédiatement sur tout symbole non reconnu.

```rust
// parser/src/inline.rs

use crate::{
    ast::{ContainerNode, ContainerType, InlineNode, NodeBase, Span, TextNode},
    error::{ParseError, SourcePosition},
    id::IdGenerator,
};

/// Symboles Markdown reconnus, du plus long au plus court.
/// L'ordre est important : `**` doit être testé avant `*`.
const MARKDOWN_MARKERS: &[(&str, ContainerType)] = &[
    ("**",  ContainerType::Bold),
    ("*",   ContainerType::Italic),
    ("_",   ContainerType::Italic),
    ("~~",  ContainerType::Strikethrough),
    ("==",  ContainerType::Surline),
];

/// Parse une chaîne inline en une liste de `InlineNode`.
///
/// `parent_type` : type du conteneur parent, utilisé pour le style des `TextNode`.
/// `line`        : numéro de ligne courant (pour les messages d'erreur).
/// `line_start`  : indice d'octet du début de la ligne courante.
pub fn parse_inline(
    input: &str,
    line: usize,
    line_start: usize,
    id_gen: &mut IdGenerator,
) -> Result<Vec<InlineNode>, ParseError> {
    let mut nodes: Vec<InlineNode> = Vec::new();
    let mut current_text = String::new();
    let mut current_text_start = line_start;
    let mut chars = input.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        let abs_offset = line_start + i;

        // Détection d'un marqueur connu (du plus long au plus court)
        let remaining = &input[i..];
        if let Some((marker, container_type)) = detect_marker(remaining) {
            // Flush le texte accumulé avant le marqueur
            if !current_text.is_empty() {
                nodes.push(make_text(&current_text, current_text_start, abs_offset, id_gen));
                current_text.clear();
            }

            let marker_len = marker.len();
            let after_open = &input[i + marker_len..];

            // Recherche du marqueur fermant
            match after_open.find(marker) {
                Some(close_rel) => {
                    let inner_start = i + marker_len;
                    let inner_end   = inner_start + close_rel;
                    let inner = &input[inner_start..inner_end];

                    // Récursion pour les styles imbriqués
                    let children = parse_inline(inner, line, line_start + inner_start, id_gen)?;
                    let span_end = line_start + inner_end + marker_len;
                    nodes.push(InlineNode::Container(ContainerNode {
                        base: NodeBase::new(id_gen.next(), Span::new(abs_offset, span_end)),
                        container_type,
                        children,
                    }));

                    // Avance l'itérateur jusqu'après le marqueur fermant
                    let skip = marker_len - 1 + close_rel + marker_len;
                    for _ in 0..skip { chars.next(); }
                    current_text_start = span_end;
                }
                None => {
                    // Marqueur ouvrant sans fermeture → symbole non supporté
                    return Err(ParseError::UnsupportedSymbol {
                        symbol: marker.to_string(),
                        position: SourcePosition {
                            line,
                            column: i + 1,
                            byte_offset: abs_offset,
                        },
                    });
                }
            }
        } else if c == '`' || c == '<' {
            // Symbole explicitement non supporté en Markdown
            return Err(ParseError::UnsupportedSymbol {
                symbol: extract_symbol(input, i),
                position: SourcePosition {
                    line,
                    column: i + 1,
                    byte_offset: abs_offset,
                },
            });
        } else if c == '\n' {
            current_text.push('\n');
        } else {
            current_text.push(c);
        }
    }

    // Flush le texte restant
    if !current_text.is_empty() {
        let end = line_start + input.len();
        nodes.push(make_text(&current_text, current_text_start, end, id_gen));
    }

    Ok(nodes)
}

fn make_text(text: &str, start: usize, end: usize, id_gen: &mut IdGenerator) -> InlineNode {
    InlineNode::Text(TextNode {
        base: NodeBase::new(id_gen.next(), Span::new(start, end)),
        text: text.to_string(),
    })
}

/// Détecte le marqueur le plus long qui commence à `s`.
fn detect_marker(s: &str) -> Option<(&'static str, ContainerType)> {
    for &(marker, ref ct) in MARKDOWN_MARKERS {
        if s.starts_with(marker) {
            return Some((marker, ct.clone()));
        }
    }
    None
}

/// Extrait le symbole ou tag lisible pour le message d'erreur.
/// Ex: "<div>", "<span class=...>" → "<div>", "`code`" → "`".
fn extract_symbol(s: &str, from: usize) -> String {
    let rest = &s[from..];
    if rest.starts_with('<') {
        // Extrait jusqu'au '>' inclus, ou tout ce qui reste
        let end = rest.find('>').map(|i| i + 1).unwrap_or(rest.len());
        rest[..end].to_string()
    } else {
        rest.chars().next().map(|c| c.to_string()).unwrap_or_default()
    }
}
```

---

## `MarkdownParser`

```rust
// parser/src/markdown.rs

use crate::{
    ast::{ContainerNode, ContainerType, InlineNode, ListItemNode, NodeBase, Span},
    error::{ParseError, SourcePosition},
    id::IdGenerator,
    inline::parse_inline,
    Parser,
};

pub struct MarkdownParser;

impl Parser for MarkdownParser {
    fn parse(&self, input: &str) -> Result<Vec<ContainerNode>, ParseError> {
        const MAX_SIZE: usize = 10 * 1024 * 1024;
        if input.len() > MAX_SIZE {
            return Err(ParseError::InputTooLarge { found: input.len(), max: MAX_SIZE });
        }

        let mut nodes: Vec<ContainerNode> = Vec::new();
        let mut id_gen = IdGenerator::new();
        let mut byte_offset: usize = 0;

        for (line_idx, line) in input.lines().enumerate() {
            let line_num = line_idx + 1;
            let line_start = byte_offset;
            byte_offset += line.len() + 1;
            let span = Span::new(line_start, byte_offset.min(input.len()));
            let trimmed = line.trim();

            if trimmed.is_empty() { continue; }

            // Titres Markdown (#, ##, ...) : non supportés en Unicode
            if trimmed.starts_with('#') {
                let hashes: String = trimmed.chars().take_while(|&c| c == '#').collect();
                return Err(ParseError::UnsupportedSymbol {
                    symbol: hashes,
                    position: SourcePosition { line: line_num, column: 1, byte_offset: line_start },
                });
            }

            // Citation
            if let Some(rest) = trimmed.strip_prefix("> ") {
                let children = parse_inline(rest, line_num, line_start, &mut id_gen)?;
                nodes.push(ContainerNode {
                    base: NodeBase::new(id_gen.next(), span),
                    container_type: ContainerType::Blockquote,
                    children,
                });
                continue;
            }

            // Liste non ordonnée
            if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
                let item = build_list_item(rest, line_num, line_start, &mut id_gen)?;
                nodes.push(ContainerNode {
                    base: NodeBase::new(id_gen.next(), span),
                    container_type: ContainerType::List,
                    children: vec![InlineNode::ListItem(item)],
                });
                continue;
            }

            // Liste ordonnée
            if let Some(rest) = try_strip_ordered_prefix(trimmed) {
                let item = build_list_item(rest, line_num, line_start, &mut id_gen)?;
                nodes.push(ContainerNode {
                    base: NodeBase::new(id_gen.next(), span),
                    container_type: ContainerType::OrderedList,
                    children: vec![InlineNode::ListItem(item)],
                });
                continue;
            }

            // Texte ordinaire (paragraphe)
            let children = parse_inline(trimmed, line_num, line_start, &mut id_gen)?;
            nodes.push(ContainerNode {
                base: NodeBase::new(id_gen.next(), span),
                container_type: ContainerType::Text,
                children,
            });
        }

        Ok(nodes)
    }
}

    fn supported_symbols(&self) -> Vec<SupportedSymbol> {
        vec![
            SupportedSymbol { symbol: "**".into(), description: "Gras".into(),          example: "**texte**".into() },
            SupportedSymbol { symbol: "*".into(),  description: "Italique".into(),       example: "*texte*".into() },
            SupportedSymbol { symbol: "_".into(),  description: "Italique".into(),       example: "_texte_".into() },
            SupportedSymbol { symbol: "~~".into(), description: "Barré".into(),          example: "~~texte~~".into() },
            SupportedSymbol { symbol: "==".into(), description: "Surligné".into(),       example: "==texte==".into() },
            SupportedSymbol { symbol: "> ".into(), description: "Citation".into(),       example: "> texte".into() },
            SupportedSymbol { symbol: "- ".into(), description: "Liste".into(),          example: "- item".into() },
            SupportedSymbol { symbol: "1. ".into(),description: "Liste ordonnée".into(), example: "1. item".into() },
        ]
    }
}

fn build_list_item(
    text: &str,
    line: usize,
    line_start: usize,
    id_gen: &mut IdGenerator,
) -> Result<ListItemNode, ParseError> {
    let children = parse_inline(text, line, line_start, id_gen)?;
    Ok(ListItemNode {
        base: NodeBase::new(id_gen.next(), Span::new(line_start, line_start + text.len())),
        children,
    })
}

fn try_strip_ordered_prefix(s: &str) -> Option<&str> {
    let dot_pos = s.find(". ")?;
    let prefix = &s[..dot_pos];
    if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
        Some(&s[dot_pos + 2..])
    } else {
        None
    }
}
```

---

## `HtmlParser`

Le parser HTML est strict et **sans dépendance externe**. Il parcourt le HTML caractère par caractère, exactement comme le parser Markdown, en utilisant une stack explicite pour gérer l'imbrication des tags.

> L'algorithme complet est documenté dans **`03b-parser-html.md`**.

```rust
// parser/src/html.rs

use crate::{
    ast::{ContainerNode, ContainerType, InlineNode, ListItemNode, NodeBase, Span, TextNode},
    error::{ParseError, SourcePosition},
    id::IdGenerator,
    Parser,
};

/// Tags HTML supportés. Tout tag absent → ParseError::UnsupportedSymbol immédiat.
/// Les titres <h1>–<h6> ne sont pas supportés : pas d'équivalent Unicode.
const SUPPORTED_TAGS: &[&str] = &[
    "strong", "b", "em", "i", "u", "s", "del", "mark",
    "blockquote", "ul", "ol", "li",
    "br", "p",
];

/// Contexte empilé lors de l'ouverture d'un tag.
struct OpenTag {
    container_type: ContainerType,
    /// Nom original du tag (ex: "b", "strong") pour valider le tag fermant.
    tag_name: String,
    children: Vec<InlineNode>,
    opened_at: SourcePosition,
}

pub struct HtmlParser;

impl Parser for HtmlParser {
    fn parse(&self, input: &str) -> Result<Vec<ContainerNode>, ParseError> {
        const MAX_SIZE: usize = 10 * 1024 * 1024;
        if input.len() > MAX_SIZE {
            return Err(ParseError::InputTooLarge { found: input.len(), max: MAX_SIZE });
        }

        let mut stack: Vec<OpenTag> = Vec::new();
        let mut nodes: Vec<InlineNode> = Vec::new();
        let mut current_text = String::new();
        let mut current_text_start = 0usize;
        let mut id_gen = IdGenerator::new();
        let mut line = 1usize;
        let mut line_start = 0usize;
        let mut i = 0usize;

        while i < input.len() {
            let c = input[i..].chars().next().unwrap();

            if c == '\n' {
                current_text.push('\n');
                line += 1;
                line_start = i + 1;
                i += 1;
                continue;
            }

            if c != '<' {
                current_text.push(c);
                i += c.len_utf8();
                continue;
            }

            // c == '<' : début d'un tag
            let position = SourcePosition { line, column: i - line_start + 1, byte_offset: i };

            match extract_tag(input, i) {
                None => {
                    // Commentaire <!-- --> ou DOCTYPE : avance jusqu'au '>'
                    if let Some(end) = input[i..].find('>') {
                        i += end + 1;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                Some((is_closing, tag_name, tag_len)) => {
                    let tag_lower = tag_name.to_lowercase();

                    // Vérifie que le tag est supporté
                    if !SUPPORTED_TAGS.contains(&tag_lower.as_str()) {
                        let symbol = format!("<{}{}>",
                            if is_closing { "/" } else { "" }, tag_name);
                        return Err(ParseError::UnsupportedSymbol { symbol, position });
                    }

                    if is_closing {
                        // Tag fermant
                        // Flush le texte courant dans le niveau courant
                        flush_text(&mut current_text, &mut current_text_start,
                                   i, current_children(&mut stack, &mut nodes), &mut id_gen);

                        match tag_lower.as_str() {
                            "br" => { /* void tag, pas de fermeture réelle */ }
                            "p" => {
                                // </p> → insère un \n dans le niveau courant
                                let target = current_children(&mut stack, &mut nodes);
                                target.push(InlineNode::Text(TextNode {
                                    base: NodeBase::new(id_gen.next(), Span::new(i, i + tag_len)),
                                    text: "\n".to_string(),
                                }));
                            }
                            _ => {
                                // Dépile et rattache au niveau parent
                                pop_tag(&mut stack, &mut nodes, &tag_lower,
                                        position, &mut id_gen)?;
                            }
                        }
                    } else {
                        // Tag ouvrant
                        // Flush le texte courant dans le niveau courant
                        flush_text(&mut current_text, &mut current_text_start,
                                   i, current_children(&mut stack, &mut nodes), &mut id_gen);

                        match tag_lower.as_str() {
                            "br" => {
                                // Void tag : insère directement un \n
                                let target = current_children(&mut stack, &mut nodes);
                                target.push(InlineNode::Text(TextNode {
                                    base: NodeBase::new(id_gen.next(), Span::new(i, i + tag_len)),
                                    text: "\n".to_string(),
                                }));
                            }
                            "p" => {
                                // Transparent : pas d'empilement, le </p> ajoutera le \n
                            }
                            _ => {
                                // Empile le tag ouvrant
                                let container_type = tag_to_container_type(&tag_lower).unwrap();
                                stack.push(OpenTag {
                                    container_type,
                                    tag_name: tag_lower,
                                    children: Vec::new(),
                                    opened_at: position,
                                });
                            }
                        }
                    }

                    current_text_start = i + tag_len;
                    i += tag_len;
                }
            }
        }

        // Flush le texte restant
        flush_text(&mut current_text, &mut current_text_start,
                   input.len(), &mut nodes, &mut id_gen);

        // Stack non vide = tag ouvert non fermé
        if let Some(unclosed) = stack.into_iter().next() {
            return Err(ParseError::UnsupportedSymbol {
                symbol: format!("<{}>", unclosed.tag_name),
                position: unclosed.opened_at,
            });
        }

        // Remonte les InlineNode racines en ContainerNode de niveau bloc
        Ok(inline_nodes_to_block(nodes, &mut id_gen))
    }
}

/// Extrait (is_closing, tag_name, longueur_totale) depuis la position du '<'.
/// Retourne None pour les commentaires et DOCTYPE.
fn extract_tag(input: &str, from: usize) -> Option<(bool, String, usize)> {
    let rest = &input[from + 1..];
    if rest.starts_with('!') || rest.starts_with('?') {
        return None;
    }
    let is_closing = rest.starts_with('/');
    let name_offset = if is_closing { 1 } else { 0 };
    let tag_name: String = rest[name_offset..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    if tag_name.is_empty() { return None; }
    let tag_end = input[from..].find('>')?;
    Some((is_closing, tag_name, tag_end + 1))
}

/// Retourne une référence mutable vers le Vec<InlineNode> du niveau courant.
fn current_children<'a>(
    stack: &'a mut Vec<OpenTag>,
    nodes: &'a mut Vec<InlineNode>,
) -> &'a mut Vec<InlineNode> {
    if let Some(top) = stack.last_mut() {
        &mut top.children
    } else {
        nodes
    }
}

/// Flush le tampon de texte dans la cible si non vide (filtre le whitespace seul).
fn flush_text(
    text: &mut String,
    start: &mut usize,
    end: usize,
    target: &mut Vec<InlineNode>,
    id_gen: &mut IdGenerator,
) {
    if !text.trim().is_empty() {
        target.push(InlineNode::Text(TextNode {
            base: NodeBase::new(id_gen.next(), Span::new(*start, end)),
            text: std::mem::take(text),
        }));
    } else {
        text.clear();
    }
    *start = end;
}

/// Dépile le tag fermant, vérifie la cohérence, et rattache au niveau parent.
fn pop_tag(
    stack: &mut Vec<OpenTag>,
    nodes: &mut Vec<InlineNode>,
    tag_name: &str,
    position: SourcePosition,
    id_gen: &mut IdGenerator,
) -> Result<(), ParseError> {
    let top = stack.last().ok_or_else(|| ParseError::UnsupportedSymbol {
        symbol: format!("</{}>", tag_name),
        position: position.clone(),
    })?;

    if top.tag_name != tag_name {
        return Err(ParseError::UnsupportedSymbol {
            symbol: format!("</{}>", tag_name),
            position,
        });
    }

    let open_tag = stack.pop().unwrap();
    let node = InlineNode::Container(ContainerNode {
        base: NodeBase::new(id_gen.next(), Span::new(0, 0)),
        container_type: open_tag.container_type,
        children: open_tag.children,
    });

    current_children(stack, nodes).push(node);
    Ok(())
}

/// Correspondance tag HTML → ContainerType.
fn tag_to_container_type(tag: &str) -> Option<ContainerType> {
    match tag {
        "strong" | "b" => Some(ContainerType::Bold),
        "em" | "i"     => Some(ContainerType::Italic),
        "u"            => Some(ContainerType::Underline),
        "s" | "del"    => Some(ContainerType::Strikethrough),
        "mark"         => Some(ContainerType::Surline),
        "blockquote"   => Some(ContainerType::Blockquote),
        "ul"           => Some(ContainerType::List),
        "ol"           => Some(ContainerType::OrderedList),
        "li"           => Some(ContainerType::ListItem),
        _              => None,
    }
}

impl Parser for HtmlParser {
    // parse() : voir ci-dessus

    fn supported_symbols(&self) -> Vec<SupportedSymbol> {
        vec![
            SupportedSymbol { symbol: "<strong>".into(), description: "Gras".into(),          example: "<strong>texte</strong>".into() },
            SupportedSymbol { symbol: "<b>".into(),      description: "Gras".into(),          example: "<b>texte</b>".into() },
            SupportedSymbol { symbol: "<em>".into(),     description: "Italique".into(),      example: "<em>texte</em>".into() },
            SupportedSymbol { symbol: "<i>".into(),      description: "Italique".into(),      example: "<i>texte</i>".into() },
            SupportedSymbol { symbol: "<u>".into(),      description: "Souligné".into(),      example: "<u>texte</u>".into() },
            SupportedSymbol { symbol: "<s>".into(),      description: "Barré".into(),         example: "<s>texte</s>".into() },
            SupportedSymbol { symbol: "<del>".into(),    description: "Barré".into(),         example: "<del>texte</del>".into() },
            SupportedSymbol { symbol: "<mark>".into(),   description: "Surligné".into(),      example: "<mark>texte</mark>".into() },
            SupportedSymbol { symbol: "<blockquote>".into(), description: "Citation".into(),  example: "<blockquote>texte</blockquote>".into() },
            SupportedSymbol { symbol: "<ul>".into(),     description: "Liste".into(),         example: "<ul><li>item</li></ul>".into() },
            SupportedSymbol { symbol: "<ol>".into(),     description: "Liste ordonnée".into(),example: "<ol><li>item</li></ol>".into() },
            SupportedSymbol { symbol: "<br>".into(),     description: "Saut de ligne".into(), example: "texte<br>suite".into() },
            SupportedSymbol { symbol: "<p>".into(),      description: "Paragraphe (séparateur \\n)".into(), example: "<p>texte</p>".into() },
        ]
    }
}

/// Remonte les InlineNode de niveau racine en ContainerNode de niveau bloc.
/// Les Container racines (Heading, List, etc.) deviennent des ContainerNode directs.
/// Les Text et Container inline racines sont regroupés dans un ContainerNode(Text).
fn inline_nodes_to_block(nodes: Vec<InlineNode>, id_gen: &mut IdGenerator) -> Vec<ContainerNode> {
    let mut result: Vec<ContainerNode> = Vec::new();
    let mut pending_inline: Vec<InlineNode> = Vec::new();

    for node in nodes {
        match &node {
            InlineNode::Container(c) if is_block_type(&c.container_type) => {
                // Flush les inline en attente
                if !pending_inline.is_empty() {
                    result.push(ContainerNode {
                        base: NodeBase::new(id_gen.next(), Span::new(0, 0)),
                        container_type: ContainerType::Text,
                        children: std::mem::take(&mut pending_inline),
                    });
                }
                // Remonte le ContainerNode bloc directement
                if let InlineNode::Container(c) = node {
                    result.push(c);
                }
            }
            _ => pending_inline.push(node),
        }
    }

    if !pending_inline.is_empty() {
        result.push(ContainerNode {
            base: NodeBase::new(id_gen.next(), Span::new(0, 0)),
            container_type: ContainerType::Text,
            children: pending_inline,
        });
    }

    result
}

fn is_block_type(ct: &ContainerType) -> bool {
    matches!(ct,
        ContainerType::Heading(_) |
        ContainerType::Blockquote |
        ContainerType::List |
        ContainerType::OrderedList
    )
}
```

---

## Edge cases à tester

| Cas | Entrée | Résultat attendu |
|---|---|---|
| Tag HTML non supporté | `<div>texte</div>` | `Err(UnsupportedSymbol { symbol: "<div>", line: 1, col: 1 })` |
| `<span>` explicitement refusé | `texte <span>ici</span>` | `Err(UnsupportedSymbol { symbol: "<span>", … })` |
| `<table>` refusé | `<table><tr><td>x</td></tr></table>` | `Err(UnsupportedSymbol { symbol: "<table>", … })` |
| Backtick Markdown | `` `code` `` | `Err(UnsupportedSymbol { symbol: "`", … })` |
| Marqueur Markdown non fermé | `**non fermé` | `Err(UnsupportedSymbol { symbol: "**", … })` |
| `<br>` HTML | `texte<br>suite` | `[Text("texte"), Text("\n"), Text("suite")]` |
| `</p>` | `<p>para</p>` | contenu du `<p>` + `Text("\n")` |
| Bold simple | `**gras**` | `Container(Bold) > [Text("gras")]` |
| Italic imbriqué dans bold | `**a *b* c**` | `Container(Bold) > [Text("a "), Container(Italic) > [Text("b")], Text(" c")]` |
| Surline Markdown | `==surligné==` | `Container(Surline) > [Text("surligné")]` |
| Underline HTML | `<u>souligné</u>` | `Container(Underline) > [Text("souligné")]` |
| Item de liste avec style | `- item **gras**` | `List > [ListItem > [Text("item "), Container(Bold) > [Text("gras")]]]` |
| Titre level > 6 | `####### trop` | `Err(UnsupportedSymbol { symbol: "#######", … })` |
| Contenu vide | `""` | `Ok(vec![])` |
| Entrée > 10 Mo | — | `Err(InputTooLarge)` |

---

## Ce que le parser ne fait PAS

- Il ne tolère aucun symbole ou tag absent des listes supportées.
- Il ne convertit aucun nœud en Unicode.
- Il n'importe pas `converter`.

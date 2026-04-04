# Tâche 06 — Couche Service

## Scope

Implémenter `ContentService` dans le crate `service`. Cette tâche suppose que `parser` (tâches 01–03) et `converter` (tâches 04–05) sont fonctionnels. Le service **orchestre** les deux sans connaître leurs détails internes. Les tests de cette couche **mockent** le parser et le converter via des traits — ils ne testent pas le parsing ou la conversion réels.

**Référence** : [`05-service-api-mcp.md`](../05-service-api-mcp.md), [`03-parser.md`](../03-parser.md), [`04-converter.md`](../04-converter.md)

---

## Fichiers à créer

```
service/src/
├── lib.rs       ← ContentService + re-exports
└── error.rs     ← ServiceError
```

---

## `service/src/error.rs`

```rust
use parser::ParseError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Erreur de parsing : {0}")]
    Parse(#[from] ParseError),

    #[error("Syntaxe non supportée : '{0}'. Syntaxes disponibles : markdown, html")]
    UnsupportedSyntax(String),

    #[error("Contenu manquant")]
    EmptyContent,
}
```

---

## `service/src/lib.rs`

```rust
pub mod error;
pub use error::ServiceError;

use converter::convert;
use parser::{HtmlParser, MarkdownParser, Parser, SupportedSymbol};

pub struct ContentService {
    markdown_parser: MarkdownParser,
    html_parser: HtmlParser,
}

impl ContentService {
    pub fn new() -> Self {
        Self {
            markdown_parser: MarkdownParser,
            html_parser: HtmlParser,
        }
    }

    /// Retourne les symboles supportés par le parser de la syntaxe demandée.
    /// Le parser est la seule source de vérité.
    pub fn list_syntaxes(&self, syntax: &str) -> Result<Vec<SupportedSymbol>, ServiceError> {
        match syntax.to_lowercase().as_str() {
            "markdown" | "md" => Ok(self.markdown_parser.supported_symbols()),
            "html"             => Ok(self.html_parser.supported_symbols()),
            other              => Err(ServiceError::UnsupportedSyntax(other.to_string())),
        }
    }

    /// Parse `content` avec le parser de `syntax`, puis convertit en Unicode.
    pub fn convert(&self, syntax: &str, content: &str) -> Result<String, ServiceError> {
        if content.trim().is_empty() {
            return Ok(String::new());
        }

        let nodes = match syntax.to_lowercase().as_str() {
            "markdown" | "md" => self.markdown_parser.parse(content)?,
            "html"             => self.html_parser.parse(content)?,
            other              => return Err(ServiceError::UnsupportedSyntax(other.to_string())),
        };

        Ok(convert(&nodes))
    }
}
```

---

## Tests à implémenter

Fichier : `service/src/lib.rs` (module `#[cfg(test)]`)

Les tests de cette couche vérifient **uniquement** le comportement du service (routage, gestion des erreurs, propagation). Ils utilisent des **parsers mock** pour ne pas dépendre du comportement réel des parsers.

### Mock du trait `Parser`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use parser::{ContainerNode, ContainerType, InlineNode, NodeBase, ParseError,
                 Parser, SourcePosition, Span, SupportedSymbol, TextNode};

    // Parser mock qui retourne toujours un seul TextNode
    struct MockParser {
        result: Result<Vec<ContainerNode>, ParseError>,
    }

    impl MockParser {
        fn ok(text: &str) -> Self {
            let text_node = TextNode {
                base: NodeBase::new(1, Span::new(0, text.len())),
                text: text.to_string(),
            };
            let container = ContainerNode {
                base: NodeBase::new(2, Span::new(0, text.len())),
                container_type: ContainerType::Text,
                children: vec![InlineNode::Text(text_node)],
            };
            Self { result: Ok(vec![container]) }
        }

        fn err(symbol: &str) -> Self {
            Self {
                result: Err(ParseError::UnsupportedSymbol {
                    symbol: symbol.to_string(),
                    position: SourcePosition { line: 1, column: 1, byte_offset: 0 },
                }),
            }
        }
    }

    impl Parser for MockParser {
        fn parse(&self, _input: &str) -> Result<Vec<ContainerNode>, ParseError> {
            self.result.clone()
        }
        fn supported_symbols(&self) -> Vec<SupportedSymbol> {
            vec![SupportedSymbol {
                symbol: "**".to_string(),
                description: "Gras".to_string(),
                example: "**gras**".to_string(),
            }]
        }
    }
```

### Tests du routage syntaxe

```rust
    #[test]
    fn syntaxe_markdown_route_vers_markdown_parser() {
        let svc = ContentService::new();
        // Le vrai MarkdownParser accepte du texte simple
        let result = svc.convert("markdown", "texte simple");
        assert!(result.is_ok());
    }

    #[test]
    fn alias_md_accepte() {
        let svc = ContentService::new();
        let result = svc.convert("md", "texte simple");
        assert!(result.is_ok());
    }

    #[test]
    fn syntaxe_html_route_vers_html_parser() {
        let svc = ContentService::new();
        let result = svc.convert("html", "<strong>test</strong>");
        assert!(result.is_ok());
    }

    #[test]
    fn syntaxe_inconnue_retourne_unsupported_syntax() {
        let svc = ContentService::new();
        let result = svc.convert("xml", "contenu");
        assert!(matches!(result, Err(ServiceError::UnsupportedSyntax(s)) if s == "xml"));
    }

    #[test]
    fn syntaxe_majuscule_acceptee() {
        let svc = ContentService::new();
        let result = svc.convert("Markdown", "texte simple");
        assert!(result.is_ok());
    }
```

### Tests du contenu vide

```rust
    #[test]
    fn contenu_vide_retourne_chaine_vide_sans_appel_parser() {
        let svc = ContentService::new();
        let result = svc.convert("markdown", "").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn contenu_espaces_seuls_retourne_chaine_vide() {
        let svc = ContentService::new();
        let result = svc.convert("markdown", "   \n  \t  ").unwrap();
        assert_eq!(result, "");
    }
```

### Tests de `list_syntaxes`

```rust
    #[test]
    fn list_syntaxes_markdown_retourne_symboles_non_vide() {
        let svc = ContentService::new();
        let symbols = svc.list_syntaxes("markdown").unwrap();
        assert!(!symbols.is_empty());
    }

    #[test]
    fn list_syntaxes_html_retourne_symboles_non_vide() {
        let svc = ContentService::new();
        let symbols = svc.list_syntaxes("html").unwrap();
        assert!(!symbols.is_empty());
    }

    #[test]
    fn list_syntaxes_syntaxe_inconnue_retourne_erreur() {
        let svc = ContentService::new();
        let result = svc.list_syntaxes("toml");
        assert!(matches!(result, Err(ServiceError::UnsupportedSyntax(_))));
    }

    #[test]
    fn list_syntaxes_markdown_contient_double_etoile() {
        let svc = ContentService::new();
        let symbols = svc.list_syntaxes("markdown").unwrap();
        assert!(symbols.iter().any(|s| s.symbol == "**"));
    }
```

### Propagation des erreurs du parser

```rust
    #[test]
    fn erreur_parser_propagee_comme_service_error_parse() {
        let svc = ContentService::new();
        // Le vrai MarkdownParser rejette "#"
        let result = svc.convert("markdown", "# titre");
        assert!(matches!(result, Err(ServiceError::Parse(_))));
    }
```

---

## Critère de succès

```bash
cargo test --package service
```

Tous les tests passent. `cargo check --workspace` reste vert.

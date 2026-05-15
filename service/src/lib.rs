pub mod error;
pub use error::ServiceError;

use converter::convert;
use parser::{HtmlParser, MarkdownParser, Parser, SupportedSymbol};
use tracing::{info, warn};

#[derive(Clone)]
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

    /// Returns the supported symbols for the requested syntax parser.
    pub fn list_syntaxes(&self, syntax: &str) -> Result<Vec<SupportedSymbol>, ServiceError> {
        match syntax.to_lowercase().as_str() {
            "markdown" | "md" => Ok(self.markdown_parser.supported_symbols()),
            "html" => Ok(self.html_parser.supported_symbols()),
            other => Err(ServiceError::UnsupportedSyntax(other.to_string())),
        }
    }

    /// Parses `content` with the `syntax` parser, then converts to Unicode.
    pub fn convert(&self, syntax: &str, content: &str) -> Result<String, ServiceError> {
        const MAX_SIZE: usize = 10 * 1024 * 1024;

        info!(syntax, content_len = content.len(), "convert called");

        if content.len() > MAX_SIZE {
            warn!(
                syntax,
                content_len = content.len(),
                max = MAX_SIZE,
                "input too large"
            );
            return Err(ServiceError::InputTooLarge {
                found: content.len(),
                max: MAX_SIZE,
            });
        }

        if content.trim().is_empty() {
            return Ok(String::new());
        }

        let nodes = match syntax.to_lowercase().as_str() {
            "markdown" | "md" => self.markdown_parser.parse(content)?,
            "html" => self.html_parser.parse(content)?,
            other => return Err(ServiceError::UnsupportedSyntax(other.to_string())),
        };

        let result = convert(&nodes);
        info!(syntax, output_len = result.len(), "convert succeeded");
        Ok(result)
    }
}

impl Default for ContentService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::{
        ContainerNode, ContainerType, InlineNode, NodeBase, ParseError, Parser, SourcePosition,
        Span, SupportedSymbol, TextNode,
    };

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
            Self {
                result: Ok(vec![container]),
            }
        }

        fn err(symbol: &str) -> Self {
            Self {
                result: Err(ParseError::UnsupportedSymbol {
                    symbol: symbol.to_string(),
                    position: SourcePosition {
                        line: 1,
                        column: 1,
                        byte_offset: 0,
                    },
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

    // ── Routage syntaxe ──────────────────────────────────────────────────────

    #[test]
    fn syntaxe_markdown_route_vers_markdown_parser() {
        let svc = ContentService::new();
        assert!(svc.convert("markdown", "texte simple").is_ok());
    }

    #[test]
    fn alias_md_accepte() {
        let svc = ContentService::new();
        assert!(svc.convert("md", "texte simple").is_ok());
    }

    #[test]
    fn syntaxe_html_route_vers_html_parser() {
        let svc = ContentService::new();
        assert!(svc.convert("html", "<strong>test</strong>").is_ok());
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
        assert!(svc.convert("Markdown", "texte simple").is_ok());
    }

    // ── Contenu vide ─────────────────────────────────────────────────────────

    #[test]
    fn contenu_vide_retourne_chaine_vide_sans_appel_parser() {
        let svc = ContentService::new();
        assert_eq!(svc.convert("markdown", "").unwrap(), "");
    }

    #[test]
    fn contenu_espaces_seuls_retourne_chaine_vide() {
        let svc = ContentService::new();
        assert_eq!(svc.convert("markdown", "   \n  \t  ").unwrap(), "");
    }

    // ── list_syntaxes ────────────────────────────────────────────────────────

    #[test]
    fn list_syntaxes_markdown_retourne_symboles_non_vide() {
        let svc = ContentService::new();
        assert!(!svc.list_syntaxes("markdown").unwrap().is_empty());
    }

    #[test]
    fn list_syntaxes_html_retourne_symboles_non_vide() {
        let svc = ContentService::new();
        assert!(!svc.list_syntaxes("html").unwrap().is_empty());
    }

    #[test]
    fn list_syntaxes_syntaxe_inconnue_retourne_erreur() {
        let svc = ContentService::new();
        assert!(matches!(
            svc.list_syntaxes("toml"),
            Err(ServiceError::UnsupportedSyntax(_))
        ));
    }

    #[test]
    fn list_syntaxes_markdown_contient_double_etoile() {
        let svc = ContentService::new();
        let symbols = svc.list_syntaxes("markdown").unwrap();
        assert!(symbols.iter().any(|s| s.symbol == "**"));
    }

    // ── Propagation des erreurs ───────────────────────────────────────────────

    #[test]
    fn erreur_parser_propagee_comme_service_error_parse() {
        let svc = ContentService::new();
        let result = svc.convert("markdown", "# titre");
        assert!(matches!(result, Err(ServiceError::Parse(_))));
    }

    // ── Tests via MockParser ─────────────────────────────────────────────────

    #[test]
    fn mock_ok_retourne_texte_converti() {
        let parser = MockParser::ok("hello");
        let nodes = parser.parse("ignored").unwrap();
        let result = convert(&nodes);
        assert_eq!(result, "hello");
    }

    #[test]
    fn mock_err_produit_une_erreur_parse() {
        let parser = MockParser::err("<div>");
        let err = parser.parse("ignored").unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedSymbol { .. }));
    }

    #[test]
    fn contenu_trop_grand_retourne_input_too_large() {
        let svc = ContentService::new();
        let huge = "a".repeat(10 * 1024 * 1024 + 1);
        let result = svc.convert("markdown", &huge);
        assert!(matches!(result, Err(ServiceError::InputTooLarge { .. })));
    }

    #[test]
    fn contenu_a_la_limite_est_accepte() {
        let svc = ContentService::new();
        let at_limit = "a".repeat(10 * 1024 * 1024);
        let result = svc.convert("markdown", &at_limit);
        assert!(result.is_ok());
    }
}

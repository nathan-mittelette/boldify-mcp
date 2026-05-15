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
                description: "Bold".to_string(),
                example: "**bold**".to_string(),
            }]
        }
    }

    // ── Syntax routing ───────────────────────────────────────────────────────

    #[test]
    fn markdown_syntax_routes_to_markdown_parser() {
        let svc = ContentService::new();
        assert!(svc.convert("markdown", "simple text").is_ok());
    }

    #[test]
    fn md_alias_is_accepted() {
        let svc = ContentService::new();
        assert!(svc.convert("md", "simple text").is_ok());
    }

    #[test]
    fn html_syntax_routes_to_html_parser() {
        let svc = ContentService::new();
        assert!(svc.convert("html", "<strong>test</strong>").is_ok());
    }

    #[test]
    fn unknown_syntax_returns_unsupported_syntax_error() {
        let svc = ContentService::new();
        let result = svc.convert("xml", "content");
        assert!(matches!(result, Err(ServiceError::UnsupportedSyntax(s)) if s == "xml"));
    }

    #[test]
    fn uppercase_syntax_is_accepted() {
        let svc = ContentService::new();
        assert!(svc.convert("Markdown", "simple text").is_ok());
    }

    // ── Empty content ────────────────────────────────────────────────────────

    #[test]
    fn empty_content_returns_empty_string_without_calling_parser() {
        let svc = ContentService::new();
        assert_eq!(svc.convert("markdown", "").unwrap(), "");
    }

    #[test]
    fn whitespace_only_content_returns_empty_string() {
        let svc = ContentService::new();
        assert_eq!(svc.convert("markdown", "   \n  \t  ").unwrap(), "");
    }

    // ── list_syntaxes ────────────────────────────────────────────────────────

    #[test]
    fn list_syntaxes_markdown_returns_non_empty_symbols() {
        let svc = ContentService::new();
        assert!(!svc.list_syntaxes("markdown").unwrap().is_empty());
    }

    #[test]
    fn list_syntaxes_html_returns_non_empty_symbols() {
        let svc = ContentService::new();
        assert!(!svc.list_syntaxes("html").unwrap().is_empty());
    }

    #[test]
    fn list_syntaxes_unknown_syntax_returns_error() {
        let svc = ContentService::new();
        assert!(matches!(
            svc.list_syntaxes("toml"),
            Err(ServiceError::UnsupportedSyntax(_))
        ));
    }

    #[test]
    fn list_syntaxes_markdown_contains_double_star() {
        let svc = ContentService::new();
        let symbols = svc.list_syntaxes("markdown").unwrap();
        assert!(symbols.iter().any(|s| s.symbol == "**"));
    }

    // ── Error propagation ────────────────────────────────────────────────────

    #[test]
    fn parser_error_propagated_as_service_error_parse() {
        let svc = ContentService::new();
        let result = svc.convert("markdown", "# title");
        assert!(matches!(result, Err(ServiceError::Parse(_))));
    }

    // ── MockParser tests ─────────────────────────────────────────────────────

    #[test]
    fn mock_ok_returns_converted_text() {
        let parser = MockParser::ok("hello");
        let nodes = parser.parse("ignored").unwrap();
        let result = convert(&nodes);
        assert_eq!(result, "hello");
    }

    #[test]
    fn mock_err_produces_parse_error() {
        let parser = MockParser::err("<div>");
        let err = parser.parse("ignored").unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedSymbol { .. }));
    }

    #[test]
    fn content_too_large_returns_input_too_large_error() {
        let svc = ContentService::new();
        let huge = "a".repeat(10 * 1024 * 1024 + 1);
        let result = svc.convert("markdown", &huge);
        assert!(matches!(result, Err(ServiceError::InputTooLarge { .. })));
    }

    #[test]
    fn content_at_limit_is_accepted() {
        let svc = ContentService::new();
        let at_limit = "a".repeat(10 * 1024 * 1024);
        let result = svc.convert("markdown", &at_limit);
        assert!(result.is_ok());
    }

    #[test]
    fn concurrent_conversions_are_consistent() {
        use std::sync::Arc;
        let svc = Arc::new(ContentService::new());
        let handles: Vec<_> = (0..20)
            .map(|_| {
                let svc = Arc::clone(&svc);
                std::thread::spawn(move || svc.convert("markdown", "**bold**").unwrap())
            })
            .collect();
        let expected = svc.convert("markdown", "**bold**").unwrap();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), expected);
        }
    }
}

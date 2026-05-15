use std::fmt;

const SYNTAXES_HTTP_ENDPOINT: &str = "GET /syntaxes";
const SYNTAXES_MCP_COMMAND: &str = "mcp list_syntaxes";

/// Position of a token in the source input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {} (offset {})",
            self.line, self.column, self.byte_offset
        )
    }
}

/// Errors produced by the Markdown and HTML parsers.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error(
        "Unsupported symbol: `{symbol}` at {position}.\n\
         Check the list of supported syntax via:\n\
         - HTTP API : {endpoint}\n\
         - MCP CLI  : {mcp_cmd}",
        endpoint = SYNTAXES_HTTP_ENDPOINT,
        mcp_cmd = SYNTAXES_MCP_COMMAND
    )]
    UnsupportedSymbol {
        symbol: String,
        position: SourcePosition,
    },

    /// The HTML document is structurally invalid.
    #[error("Invalid HTML document: {0}")]
    InvalidHtml(String),

    /// The input exceeds the maximum allowed size.
    #[error("Input too large: {found} bytes (maximum: {max})")]
    InputTooLarge { found: usize, max: usize },

    /// An HTML opening tag was never closed.
    #[error("Unclosed HTML tag: `{tag}` opened at {position}")]
    UnclosedTag {
        tag: String,
        position: SourcePosition,
    },

    /// The AST nesting depth exceeded the allowed maximum.
    #[error("Nesting too deep: depth {depth} (maximum: {max})")]
    NestingTooDeep { depth: usize, max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_symbol_error_contains_symbol() {
        let err = ParseError::UnsupportedSymbol {
            symbol: "<div>".to_string(),
            position: SourcePosition {
                line: 3,
                column: 7,
                byte_offset: 42,
            },
        };
        let msg = err.to_string();
        assert!(msg.contains("<div>"));
        assert!(msg.contains("GET /syntaxes"));
    }

    #[test]
    fn source_position_displays_line_and_column() {
        let pos = SourcePosition {
            line: 2,
            column: 5,
            byte_offset: 20,
        };
        assert!(pos.to_string().contains("line 2"));
        assert!(pos.to_string().contains("column 5"));
    }
}

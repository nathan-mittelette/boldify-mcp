use parser::ParseError;

/// Errors returned by [`super::ContentService`].
#[derive(Debug, Clone, thiserror::Error)]
pub enum ServiceError {
    /// A parse error propagated from the underlying parser.
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    /// The requested syntax is not supported.
    #[error("Unsupported syntax: '{0}'. Available syntaxes: markdown, html")]
    UnsupportedSyntax(String),

    /// The input exceeds the maximum allowed size.
    #[error("Input too large: {found} bytes (maximum: {max})")]
    InputTooLarge { found: usize, max: usize },
}

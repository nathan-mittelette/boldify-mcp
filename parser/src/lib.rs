//! Parser crate — converts Markdown or HTML source text into an AST.
//!
//! Use [`MarkdownParser`] or [`HtmlParser`] via the [`Parser`] trait.
//! Unsupported constructs produce [`ParseError::UnsupportedSymbol`].

pub mod ast;
pub mod error;
pub mod html;
pub mod id;
pub mod inline;
pub mod markdown;

pub use ast::*;
pub use error::{ParseError, SourcePosition};
pub use html::HtmlParser;
pub use markdown::MarkdownParser;

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

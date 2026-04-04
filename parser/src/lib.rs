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

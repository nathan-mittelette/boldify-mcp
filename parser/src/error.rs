use std::fmt;

const SYNTAXES_HTTP_ENDPOINT: &str = "GET /syntaxes";
const SYNTAXES_MCP_COMMAND: &str = "mcp list";

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
            "ligne {}, colonne {} (offset {})",
            self.line, self.column, self.byte_offset
        )
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error(
        "Symbole non supporté : `{symbol}` à la position {position}.\n\
         Consultez la liste des syntaxes supportées via :\n\
         - API HTTP : {endpoint}\n\
         - MCP CLI  : {mcp_cmd}",
        endpoint = SYNTAXES_HTTP_ENDPOINT,
        mcp_cmd = SYNTAXES_MCP_COMMAND
    )]
    UnsupportedSymbol {
        symbol: String,
        position: SourcePosition,
    },

    #[error("Document HTML invalide : {0}")]
    InvalidHtml(String),

    #[error("Entrée trop volumineuse : {found} octets (maximum : {max})")]
    InputTooLarge { found: usize, max: usize },

    #[error("Balise HTML non fermée : `{tag}` ouverte à {position}")]
    UnclosedTag {
        tag: String,
        position: SourcePosition,
    },

    #[error("Imbrication trop profonde : profondeur {depth} (maximum : {max})")]
    NestingTooDeep { depth: usize, max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_unsupported_affiche_le_symbole() {
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
    fn source_position_affiche_ligne_et_colonne() {
        let pos = SourcePosition {
            line: 2,
            column: 5,
            byte_offset: 20,
        };
        assert!(pos.to_string().contains("ligne 2"));
        assert!(pos.to_string().contains("colonne 5"));
    }
}

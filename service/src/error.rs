use parser::ParseError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ServiceError {
    #[error("Erreur de parsing : {0}")]
    Parse(#[from] ParseError),

    #[error("Syntaxe non supportée : '{0}'. Syntaxes disponibles : markdown, html")]
    UnsupportedSyntax(String),

    #[error("Contenu trop volumineux : {found} octets (maximum : {max})")]
    InputTooLarge { found: usize, max: usize },
}

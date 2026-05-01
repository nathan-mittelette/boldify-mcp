use once_cell::sync::OnceCell;

use super::handler::FontHandler;

pub struct SurlineHandler;

static INSTANCE: OnceCell<SurlineHandler> = OnceCell::new();

pub fn surline_handler() -> &'static SurlineHandler {
    INSTANCE.get_or_init(|| SurlineHandler)
}

impl FontHandler for SurlineHandler {
    fn apply(&self, text: &str) -> String {
        format!("〚{}〛", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surline_encadre_avec_crochets_unicode() {
        let result = SurlineHandler.apply("texte");
        assert!(result.starts_with('〚'));
        assert!(result.ends_with('〛'));
        assert!(result.contains("texte"));
    }

    #[test]
    fn surline_preserve_contenu() {
        let result = SurlineHandler.apply("Hello World");
        assert!(result.contains("Hello World"));
    }

    #[test]
    fn surline_texte_vide() {
        let result = SurlineHandler.apply("");
        assert_eq!(result, "〚〛");
    }

    // ── Tâche 05b ────────────────────────────────────────────────────────────

    #[test]
    fn surline_encadre_avec_guillemets_speciaux() {
        let result = SurlineHandler.apply("texte");
        assert!(result.starts_with('〚'));
        assert!(result.ends_with('〛'));
    }

    #[test]
    fn surline_avec_emoji() {
        let result = SurlineHandler.apply("🚀 go");
        assert!(result.contains("🚀 go"));
    }
}

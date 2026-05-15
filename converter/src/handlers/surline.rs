use once_cell::sync::OnceCell;

use super::handler::FontHandler;
use super::shared::apply_combining_mark;

pub struct SurlineHandler;

static INSTANCE: OnceCell<SurlineHandler> = OnceCell::new();

pub fn surline_handler() -> &'static SurlineHandler {
    INSTANCE.get_or_init(|| SurlineHandler)
}

impl FontHandler for SurlineHandler {
    fn apply(&self, text: &str) -> String {
        apply_combining_mark(text, "\u{0305}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surline_ajoute_combining_overline_a_chaque_char() {
        let result = SurlineHandler.apply("AB");
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[1], '\u{0305}');
        assert_eq!(chars[3], '\u{0305}');
    }

    #[test]
    fn surline_preserve_espace() {
        assert_eq!(SurlineHandler.apply(" "), " ");
    }

    #[test]
    fn surline_preserve_newline() {
        assert_eq!(SurlineHandler.apply("\n"), "\n");
    }

    #[test]
    fn surline_avec_emoji() {
        let result = SurlineHandler.apply("🚀 go");
        assert!(result.contains("🚀"));
        assert!(result.contains(' '));
    }

    #[test]
    fn surline_texte_vide() {
        let result = SurlineHandler.apply("");
        assert_eq!(result, "");
    }
}

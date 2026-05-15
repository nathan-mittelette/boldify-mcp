use once_cell::sync::OnceCell;

use super::handler::FontHandler;
use super::shared::apply_combining_mark;

pub struct StrikethroughHandler;

static INSTANCE: OnceCell<StrikethroughHandler> = OnceCell::new();

pub fn strikethrough_handler() -> &'static StrikethroughHandler {
    INSTANCE.get_or_init(|| StrikethroughHandler)
}

impl FontHandler for StrikethroughHandler {
    fn apply(&self, text: &str) -> String {
        apply_combining_mark(text, "\u{0336}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::underline::UnderlineHandler;

    #[test]
    fn strikethrough_ajoute_combining_stroke() {
        let result = StrikethroughHandler.apply("X");
        assert!(result.contains('\u{0336}'));
    }

    #[test]
    fn strikethrough_preserve_espace() {
        assert_eq!(StrikethroughHandler.apply(" "), " ");
    }

    #[test]
    fn strikethrough_preserve_newline() {
        assert_eq!(StrikethroughHandler.apply("\n"), "\n");
    }

    #[test]
    fn strikethrough_chaque_char_a_son_combining_stroke() {
        let result = StrikethroughHandler.apply("AB");
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[1], '\u{0336}');
        assert_eq!(chars[3], '\u{0336}');
    }

    #[test]
    fn strikethrough_different_du_underline() {
        let u = UnderlineHandler.apply("X");
        let s = StrikethroughHandler.apply("X");
        assert_ne!(u, s);
    }

    #[test]
    fn strikethrough_preserve_espace_dans_phrase() {
        let result = StrikethroughHandler.apply("A B");
        assert!(result.contains(' '));
    }
}

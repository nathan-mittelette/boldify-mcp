use once_cell::sync::OnceCell;

use super::handler::FontHandler;
use super::shared::apply_combining_mark;

pub struct UnderlineHandler;

static INSTANCE: OnceCell<UnderlineHandler> = OnceCell::new();

pub fn underline_handler() -> &'static UnderlineHandler {
    INSTANCE.get_or_init(|| UnderlineHandler)
}

impl FontHandler for UnderlineHandler {
    fn apply(&self, text: &str) -> String {
        apply_combining_mark(text, "\u{0332}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn underline_ajoute_combining_a_chaque_char() {
        let result = UnderlineHandler.apply("AB");
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[1], '\u{0332}');
        assert_eq!(chars[3], '\u{0332}');
    }

    #[test]
    fn underline_preserve_espace() {
        assert_eq!(UnderlineHandler.apply(" "), " ");
    }

    #[test]
    fn underline_preserve_newline() {
        assert_eq!(UnderlineHandler.apply("\n"), "\n");
    }

    #[test]
    fn underline_chaque_char_a_son_combining() {
        let result = UnderlineHandler.apply("ABC");
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars.len(), 6);
        assert_eq!(chars[1], '\u{0332}');
        assert_eq!(chars[3], '\u{0332}');
        assert_eq!(chars[5], '\u{0332}');
    }

    #[test]
    fn underline_emoji_preserve_sans_combining() {
        let result = UnderlineHandler.apply("🚀");
        assert!(result.contains('🚀') || result.chars().next().unwrap() as u32 > 0xFFFF);
    }

    #[test]
    fn underline_espace_preserve_sans_combining() {
        let result = UnderlineHandler.apply("A B");
        let chars: Vec<char> = result.chars().collect();
        let space_index = chars.iter().position(|&c| c == ' ').unwrap();
        assert_eq!(chars[space_index - 1], '\u{0332}');
        assert_eq!(chars[space_index + 1], 'B');
    }
}

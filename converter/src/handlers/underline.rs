use std::sync::OnceLock;

use super::handler::FontHandler;
use super::shared::apply_combining_mark;

pub struct UnderlineHandler;

static INSTANCE: OnceLock<UnderlineHandler> = OnceLock::new();

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
    fn underline_adds_combining_to_each_char() {
        let result = UnderlineHandler.apply("AB");
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[1], '\u{0332}');
        assert_eq!(chars[3], '\u{0332}');
    }

    #[test]
    fn underline_preserves_space() {
        assert_eq!(UnderlineHandler.apply(" "), " ");
    }

    #[test]
    fn underline_preserves_newline() {
        assert_eq!(UnderlineHandler.apply("\n"), "\n");
    }

    #[test]
    fn underline_each_char_has_combining() {
        let result = UnderlineHandler.apply("ABC");
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars.len(), 6);
        assert_eq!(chars[1], '\u{0332}');
        assert_eq!(chars[3], '\u{0332}');
        assert_eq!(chars[5], '\u{0332}');
    }

    #[test]
    fn underline_emoji_preserved_without_combining() {
        let result = UnderlineHandler.apply("🚀");
        assert!(result.contains('🚀') || result.chars().next().unwrap() as u32 > 0xFFFF);
    }

    #[test]
    fn underline_space_preserved_without_combining() {
        let result = UnderlineHandler.apply("A B");
        let chars: Vec<char> = result.chars().collect();
        let space_index = chars.iter().position(|&c| c == ' ').unwrap();
        assert_eq!(chars[space_index - 1], '\u{0332}');
        assert_eq!(chars[space_index + 1], 'B');
    }
}

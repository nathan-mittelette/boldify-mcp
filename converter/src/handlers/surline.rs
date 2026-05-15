use std::sync::OnceLock;

use super::handler::FontHandler;
use super::shared::apply_combining_mark;

pub struct SurlineHandler;

static INSTANCE: OnceLock<SurlineHandler> = OnceLock::new();

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
    fn surline_adds_combining_overline_to_each_char() {
        let result = SurlineHandler.apply("AB");
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[1], '\u{0305}');
        assert_eq!(chars[3], '\u{0305}');
    }

    #[test]
    fn surline_preserves_space() {
        assert_eq!(SurlineHandler.apply(" "), " ");
    }

    #[test]
    fn surline_preserves_newline() {
        assert_eq!(SurlineHandler.apply("\n"), "\n");
    }

    #[test]
    fn surline_with_emoji() {
        let result = SurlineHandler.apply("🚀 go");
        assert!(result.contains("🚀"));
        assert!(result.contains(' '));
    }

    #[test]
    fn surline_empty_string() {
        let result = SurlineHandler.apply("");
        assert_eq!(result, "");
    }
}

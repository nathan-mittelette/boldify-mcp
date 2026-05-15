use std::collections::HashMap;

use std::sync::OnceLock;
use unicode_segmentation::UnicodeSegmentation;

use super::accents::decompose_accent;
use super::handler::FontHandler;

const BASE_UPPER: u32 = 0x1D5D4;
const BASE_LOWER: u32 = 0x1D5EE;
const BASE_DIGIT: u32 = 0x1D7EC;

pub struct BoldHandler {
    map: HashMap<char, String>,
}

impl BoldHandler {
    fn new() -> Self {
        let mut map = HashMap::new();

        for i in 0u32..26 {
            let normal = char::from_u32(b'A' as u32 + i).unwrap();
            let bold = char::from_u32(BASE_UPPER + i).unwrap();
            map.insert(normal, bold.to_string());
        }

        for i in 0u32..26 {
            let normal = char::from_u32(b'a' as u32 + i).unwrap();
            let bold = char::from_u32(BASE_LOWER + i).unwrap();
            map.insert(normal, bold.to_string());
        }

        for i in 0u32..10 {
            let normal = char::from_u32(b'0' as u32 + i).unwrap();
            let bold = char::from_u32(BASE_DIGIT + i).unwrap();
            map.insert(normal, bold.to_string());
        }

        let accented = [
            'é', 'è', 'ê', 'ë', 'à', 'á', 'â', 'ä', 'ù', 'ú', 'û', 'ü', 'ô', 'ö', 'î', 'ï', 'ç',
            'œ', 'æ', 'É', 'È', 'Ê', 'Ë', 'À', 'Á', 'Â', 'Ä', 'Ù', 'Ú', 'Û', 'Ü', 'Ô', 'Ö', 'Î',
            'Ï', 'Ç', 'Œ', 'Æ',
        ];
        for &c in &accented {
            if let Some((base, combining)) = decompose_accent(c) {
                if let Some(bold_base) = map.get(&base) {
                    map.insert(c, format!("{}{}", bold_base, combining));
                }
            }
        }

        Self { map }
    }
}

static INSTANCE: OnceLock<BoldHandler> = OnceLock::new();

pub fn bold_handler() -> &'static BoldHandler {
    INSTANCE.get_or_init(BoldHandler::new)
}

impl FontHandler for BoldHandler {
    fn apply(&self, text: &str) -> String {
        text.graphemes(true)
            .map(|g| {
                let mut chars = g.chars();
                if let (Some(c), None) = (chars.next(), chars.next()) {
                    self.map.get(&c).map(|s| s.as_str()).unwrap_or(g)
                } else {
                    g
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercase_a_converted_to_bold() {
        let result = bold_handler().apply("A");
        assert_ne!(result, "A");
        assert_eq!(result.chars().next().unwrap() as u32, 0x1D5D4);
    }

    #[test]
    fn lowercase_a_converted_to_bold() {
        let result = bold_handler().apply("a");
        assert_eq!(result.chars().next().unwrap() as u32, 0x1D5EE);
    }

    #[test]
    fn digit_converted_to_bold() {
        let result = bold_handler().apply("0");
        assert_eq!(result.chars().next().unwrap() as u32, 0x1D7EC);
    }

    #[test]
    fn space_is_preserved() {
        assert_eq!(bold_handler().apply(" "), " ");
    }

    #[test]
    fn newline_is_preserved() {
        assert_eq!(bold_handler().apply("\n"), "\n");
    }

    #[test]
    fn accented_e_acute_is_converted() {
        let result = bold_handler().apply("é");
        assert!(result.contains('\u{0301}'));
        assert!(!result.contains('é'));
    }

    #[test]
    fn full_word_is_converted() {
        let result = bold_handler().apply("Hello");
        assert!(!result.contains("Hello"));
        assert_eq!(result.chars().count(), 5);
    }

    #[test]
    fn singleton_returns_same_instance() {
        let h1 = bold_handler();
        let h2 = bold_handler();
        assert!(std::ptr::eq(h1, h2));
    }

    #[test]
    fn bold_full_sentence_with_spaces() {
        let result = bold_handler().apply("Hello everyone");
        assert!(!result.chars().any(|c| c.is_ascii_alphabetic()));
        assert!(result.contains(' '));
    }

    #[test]
    fn bold_sentence_with_punctuation() {
        let result = bold_handler().apply("Hello, world!");
        assert!(result.contains(','));
        assert!(result.contains('!'));
        assert!(!result.contains('H'));
        assert!(!result.contains('e'));
    }

    #[test]
    fn bold_mixed_digits_and_letters() {
        let result = bold_handler().apply("Top 3 in 2024");
        assert!(!result.contains('T'));
        assert!(!result.contains('3'));
        assert!(!result.contains('2'));
        assert!(result.contains(' '));
    }

    #[test]
    fn bold_all_accented_characters_are_transformed() {
        let accents = "éèêëàáâäùúûüôöîïçÉÈÊËÀÂÄÙÛÜÔÖÎÏÇ";
        let result = bold_handler().apply(accents);
        for c in accents.chars() {
            assert!(!result.contains(c), "Character '{}' was not transformed", c);
        }
        assert!(result.chars().any(|c| matches!(c as u32, 0x0300..=0x036F)));
    }

    #[test]
    fn bold_emoji_preserved_without_transformation() {
        assert_eq!(bold_handler().apply("🚀"), "🚀");
        assert_eq!(bold_handler().apply("🎉"), "🎉");
        assert_eq!(bold_handler().apply("👨‍💻"), "👨‍💻");
    }

    #[test]
    fn bold_text_with_emoji_in_middle() {
        let result = bold_handler().apply("Top 🚀 performer");
        assert!(result.contains("🚀"));
        assert!(!result.contains('T'));
        assert!(!result.contains('p'));
    }

    #[test]
    fn bold_newline_is_preserved() {
        assert!(bold_handler().apply("line1\nline2").contains('\n'));
    }

    #[test]
    fn bold_empty_string_returns_empty_string() {
        assert_eq!(bold_handler().apply(""), "");
    }

    #[test]
    fn bold_only_spaces_returns_spaces() {
        assert_eq!(bold_handler().apply("   "), "   ");
    }

    #[test]
    fn bold_non_latin_unicode_is_preserved() {
        assert_eq!(bold_handler().apply("你好"), "你好");
        assert_eq!(bold_handler().apply("مرحبا"), "مرحبا");
    }

    #[test]
    fn bold_all_digits() {
        let result = bold_handler().apply("0123456789");
        assert!(!result.contains('0'));
        assert!(!result.contains('9'));
        assert_eq!(result.chars().count(), 10);
    }
}

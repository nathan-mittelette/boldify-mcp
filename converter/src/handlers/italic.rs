use std::collections::HashMap;

use std::sync::OnceLock;
use unicode_segmentation::UnicodeSegmentation;

use super::accents::decompose_accent;
use super::handler::FontHandler;

const BASE_UPPER: u32 = 0x1D608;
const BASE_LOWER: u32 = 0x1D622;

pub struct ItalicHandler {
    map: HashMap<char, String>,
}

impl ItalicHandler {
    fn new() -> Self {
        let mut map = HashMap::new();

        for i in 0u32..26 {
            let normal = char::from_u32(b'A' as u32 + i).unwrap();
            let italic = char::from_u32(BASE_UPPER + i).unwrap();
            map.insert(normal, italic.to_string());
        }

        for i in 0u32..26 {
            let normal = char::from_u32(b'a' as u32 + i).unwrap();
            let italic = char::from_u32(BASE_LOWER + i).unwrap();
            map.insert(normal, italic.to_string());
        }

        // No italic digit variant in Unicode — digits are kept as ASCII.

        let accented = [
            'é', 'è', 'ê', 'ë', 'à', 'á', 'â', 'ä', 'ù', 'ú', 'û', 'ü', 'ô', 'ö', 'î', 'ï', 'ç',
            'œ', 'æ', 'É', 'È', 'Ê', 'Ë', 'À', 'Á', 'Â', 'Ä', 'Ù', 'Ú', 'Û', 'Ü', 'Ô', 'Ö', 'Î',
            'Ï', 'Ç', 'Œ', 'Æ',
        ];
        for &c in &accented {
            if let Some((base, combining)) = decompose_accent(c) {
                if let Some(italic_base) = map.get(&base) {
                    map.insert(c, format!("{}{}", italic_base, combining));
                }
            }
        }

        Self { map }
    }
}

static INSTANCE: OnceLock<ItalicHandler> = OnceLock::new();

pub fn italic_handler() -> &'static ItalicHandler {
    INSTANCE.get_or_init(ItalicHandler::new)
}

impl FontHandler for ItalicHandler {
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
    fn a_majuscule_converti_en_italic() {
        let result = italic_handler().apply("A");
        assert_ne!(result, "A");
        assert_eq!(result.chars().next().unwrap() as u32, 0x1D608);
    }

    #[test]
    fn a_minuscule_converti_en_italic() {
        let result = italic_handler().apply("a");
        assert_eq!(result.chars().next().unwrap() as u32, 0x1D622);
    }

    #[test]
    fn chiffre_preserve_en_ascii() {
        assert_eq!(italic_handler().apply("5"), "5");
    }

    #[test]
    fn espace_preserve() {
        assert_eq!(italic_handler().apply(" "), " ");
    }

    #[test]
    fn accent_e_aigu_converti() {
        let result = italic_handler().apply("é");
        assert!(result.contains('\u{0301}'));
        assert!(!result.contains('é'));
    }

    // ── Tâche 05b ────────────────────────────────────────────────────────────

    #[test]
    fn italic_different_du_bold() {
        let bold_result = super::super::bold::bold_handler().apply("Hello");
        let italic_result = italic_handler().apply("Hello");
        assert_ne!(bold_result, italic_result);
    }

    #[test]
    fn italic_phrase_avec_accents() {
        let result = italic_handler().apply("élégance");
        assert!(!result.contains('é'));
        assert!(!result.contains('e'));
    }

    #[test]
    fn italic_emoji_preserve() {
        assert_eq!(italic_handler().apply("✨"), "✨");
    }

    #[test]
    fn italic_chiffres_pas_de_variante_unicode() {
        assert_eq!(italic_handler().apply("123"), "123");
    }
}

# Tâche 04 — Converter : FontHandlers et accents

## Scope

Implémenter le trait `FontHandler` et toutes ses implémentations concrètes (`BoldHandler`, `ItalicHandler`, `UnderlineHandler`, `StrikethroughHandler`, `SurlineHandler`) dans le crate `converter`. Inclut la table des accents partagée. Cette tâche ne touche **ni `parser`, ni `service`** — les handlers opèrent sur des `&str` bruts, pas sur des nœuds AST.

**Référence** : [`04-converter.md`](../04-converter.md)

---

## Fichiers à créer

```
converter/src/
├── lib.rs
└── handlers/
    ├── mod.rs
    ├── handler.rs       ← trait FontHandler
    ├── bold.rs          ← BoldHandler
    ├── italic.rs        ← ItalicHandler
    ├── underline.rs     ← UnderlineHandler
    ├── strikethrough.rs ← StrikethroughHandler
    ├── surline.rs       ← SurlineHandler
    └── accents.rs       ← decompose_accent()
```

---

## Trait `FontHandler`

```rust
// converter/src/handlers/handler.rs

pub trait FontHandler: Send + Sync {
    /// Convertit du texte brut vers le style Unicode cible.
    /// Les espaces et sauts de ligne (\n) sont préservés tels quels.
    fn apply(&self, text: &str) -> String;
}
```

---

## `BoldHandler`

### Codepoints de base

```
Lettres majuscules  : U+1D5D4 (𝗔) … +25 = U+1D5ED (𝗭)
Lettres minuscules  : U+1D5EE (𝗮) … +25 = U+1D607 (𝘇)
Chiffres           : U+1D7EC (𝟬) … +9  = U+1D7F5 (𝟵)
```

### Construction de la map

```rust
// converter/src/handlers/bold.rs

use std::collections::HashMap;
use once_cell::sync::OnceCell;
use unicode_segmentation::UnicodeSegmentation;
use super::accents::decompose_accent;
use super::handler::FontHandler;

const BASE_UPPER: u32 = 0x1D5D4;
const BASE_LOWER: u32 = 0x1D5EE;
const BASE_DIGIT: u32 = 0x1D7EC;

pub struct BoldHandler {
    map: HashMap<String, String>,
}

impl BoldHandler {
    fn new() -> Self {
        let mut map = HashMap::new();

        // A–Z
        for i in 0u32..26 {
            let normal = char::from_u32(b'A' as u32 + i).unwrap();
            let bold   = char::from_u32(BASE_UPPER + i).unwrap();
            map.insert(normal.to_string(), bold.to_string());
        }

        // a–z
        for i in 0u32..26 {
            let normal = char::from_u32(b'a' as u32 + i).unwrap();
            let bold   = char::from_u32(BASE_LOWER + i).unwrap();
            map.insert(normal.to_string(), bold.to_string());
        }

        // 0–9
        for i in 0u32..10 {
            let normal = char::from_u32(b'0' as u32 + i).unwrap();
            let bold   = char::from_u32(BASE_DIGIT + i).unwrap();
            map.insert(normal.to_string(), bold.to_string());
        }

        // Accents : décomposer base + combining, styler la base
        let accented = [
            'é','è','ê','ë','à','á','â','ä','ù','ú','û','ü',
            'ô','ö','î','ï','ç','œ','æ','É','È','Ê','Ë',
            'À','Á','Â','Ä','Ù','Ú','Û','Ü','Ô','Ö','Î','Ï','Ç','Œ','Æ',
        ];
        for &c in &accented {
            if let Some((base, combining)) = decompose_accent(c) {
                if let Some(bold_base) = map.get(&base.to_string()) {
                    map.insert(c.to_string(), format!("{}{}", bold_base, combining));
                }
            }
        }

        Self { map }
    }
}

static INSTANCE: OnceCell<BoldHandler> = OnceCell::new();

pub fn bold_handler() -> &'static BoldHandler {
    INSTANCE.get_or_init(BoldHandler::new)
}

impl FontHandler for BoldHandler {
    fn apply(&self, text: &str) -> String {
        text.graphemes(true)
            .map(|g| self.map.get(g).map(|s| s.as_str()).unwrap_or(g))
            .collect()
    }
}
```

---

## Autres handlers

Même structure que `BoldHandler`. Seuls les codepoints de base changent.

### `ItalicHandler`

```
Majuscules italique : U+1D608 (𝘈) … +25
Minuscules italique : U+1D622 (𝘢) … +25
Chiffres : pas de variante italique Unicode — conserver ASCII
```

### `UnderlineHandler`

L'Unicode "souligné" s'obtient par combining character `\u{0332}` (COMBINING LOW LINE) appliqué à chaque graphème :

```rust
fn apply(&self, text: &str) -> String {
    text.graphemes(true)
        .map(|g| {
            if g == " " || g == "\n" { g.to_string() }
            else { format!("{}\u{0332}", g) }
        })
        .collect()
}
```

Pas de `map` nécessaire pour ce handler.

### `StrikethroughHandler`

Combining character `\u{0336}` (COMBINING LONG STROKE OVERLAY) :

```rust
fn apply(&self, text: &str) -> String {
    text.graphemes(true)
        .map(|g| {
            if g == " " || g == "\n" { g.to_string() }
            else { format!("{}\u{0336}", g) }
        })
        .collect()
}
```

### `SurlineHandler`

Pas d'équivalent Unicode direct pour le surlignage. Par convention, encadrer avec `〚` et `〛` :

```rust
fn apply(&self, text: &str) -> String {
    format!("〚{}〛", text)
}
```

> Note : ce choix visuel est documenté — ajuster si un autre rendu est préféré.

---

## `accents.rs`

```rust
// converter/src/handlers/accents.rs

pub fn decompose_accent(c: char) -> Option<(char, &'static str)> {
    match c {
        'é' => Some(('e', "\u{0301}")),
        'è' => Some(('e', "\u{0300}")),
        'ê' => Some(('e', "\u{0302}")),
        'ë' => Some(('e', "\u{0308}")),
        'à' => Some(('a', "\u{0300}")),
        'á' => Some(('a', "\u{0301}")),
        'â' => Some(('a', "\u{0302}")),
        'ä' => Some(('a', "\u{0308}")),
        'ù' => Some(('u', "\u{0300}")),
        'ú' => Some(('u', "\u{0301}")),
        'û' => Some(('u', "\u{0302}")),
        'ü' => Some(('u', "\u{0308}")),
        'ô' => Some(('o', "\u{0302}")),
        'ö' => Some(('o', "\u{0308}")),
        'î' => Some(('i', "\u{0302}")),
        'ï' => Some(('i', "\u{0308}")),
        'ç' => Some(('c', "\u{0327}")),
        'œ' => None, // pas de décomposition simple
        'æ' => None,
        // Majuscules
        'É' => Some(('E', "\u{0301}")),
        'È' => Some(('E', "\u{0300}")),
        'Ê' => Some(('E', "\u{0302}")),
        'Ë' => Some(('E', "\u{0308}")),
        'À' => Some(('A', "\u{0300}")),
        'Â' => Some(('A', "\u{0302}")),
        'Ä' => Some(('A', "\u{0308}")),
        'Ù' => Some(('U', "\u{0300}")),
        'Û' => Some(('U', "\u{0302}")),
        'Ü' => Some(('U', "\u{0308}")),
        'Ô' => Some(('O', "\u{0302}")),
        'Ö' => Some(('O', "\u{0308}")),
        'Î' => Some(('I', "\u{0302}")),
        'Ï' => Some(('I', "\u{0308}")),
        'Ç' => Some(('C', "\u{0327}")),
        _   => None,
    }
}
```

---

## Tests à implémenter

### `bold.rs`

```rust
#[test]
fn a_majuscule_converti_en_bold() {
    let h = bold_handler();
    let result = h.apply("A");
    assert_ne!(result, "A");
    assert_eq!(result.chars().next().unwrap() as u32, 0x1D5D4);
}

#[test]
fn a_minuscule_converti_en_bold() {
    let h = bold_handler();
    let result = h.apply("a");
    assert_eq!(result.chars().next().unwrap() as u32, 0x1D5EE);
}

#[test]
fn chiffre_converti_en_bold() {
    let h = bold_handler();
    let result = h.apply("0");
    assert_eq!(result.chars().next().unwrap() as u32, 0x1D7EC);
}

#[test]
fn espace_preserve() {
    let h = bold_handler();
    assert_eq!(h.apply(" "), " ");
}

#[test]
fn newline_preserve() {
    let h = bold_handler();
    assert_eq!(h.apply("\n"), "\n");
}

#[test]
fn accent_e_aigu_converti() {
    let h = bold_handler();
    let result = h.apply("é");
    // Doit contenir le 'e' bold + combining accent
    assert!(result.contains('\u{0301}'));
    assert!(!result.contains('é'));
}

#[test]
fn texte_complet_converti() {
    let h = bold_handler();
    let result = h.apply("Hello");
    assert!(!result.contains("Hello")); // aucun ASCII original
    assert_eq!(result.chars().count(), 5); // même nombre de graphèmes de base
}

#[test]
fn singleton_retourne_meme_instance() {
    let h1 = bold_handler();
    let h2 = bold_handler();
    assert!(std::ptr::eq(h1, h2));
}
```

### `accents.rs`

```rust
#[test]
fn decompose_e_aigu() {
    let result = decompose_accent('é');
    assert_eq!(result, Some(('e', "\u{0301}")));
}

#[test]
fn decompose_c_cedille() {
    let result = decompose_accent('ç');
    assert_eq!(result, Some(('c', "\u{0327}")));
}

#[test]
fn decompose_caractere_sans_accent_retourne_none() {
    assert_eq!(decompose_accent('z'), None);
    assert_eq!(decompose_accent('1'), None);
}
```

### `underline.rs`

```rust
#[test]
fn underline_ajoute_combining_a_chaque_char() {
    let h = UnderlineHandler;
    let result = h.apply("AB");
    // Chaque char doit être suivi de U+0332
    let chars: Vec<char> = result.chars().collect();
    assert_eq!(chars[1], '\u{0332}');
    assert_eq!(chars[3], '\u{0332}');
}

#[test]
fn underline_preserve_espace() {
    let h = UnderlineHandler;
    assert_eq!(h.apply(" "), " ");
}
```

### `strikethrough.rs`

```rust
#[test]
fn strikethrough_ajoute_combining_stroke() {
    let h = StrikethroughHandler;
    let result = h.apply("X");
    assert!(result.contains('\u{0336}'));
}
```

---

## Critère de succès

```bash
cargo test --package converter --lib handlers
```

Tous les tests passent. `cargo check --workspace` reste vert.

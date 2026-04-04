# Couche Converter

## Responsabilité

Le crate `converter` transforme un `Vec<ContainerNode>` en `String` Unicode formatée. C'est le seul endroit où le trait `ToUnicode` est défini et implémenté. Le crate `parser` n'en sait rien.

---

## Architecture : un handler par style

Chaque style de formatage est géré par une struct dédiée qui implémente le trait `FontHandler`. Cette struct :

- Construit à l'initialisation une **table de mapping unidirectionnelle** (normal → unicode stylisé uniquement).
- Gère les **accents** via combining characters (ex: `é` → `e` bold + `\u{0301}`).

```
converter/src/
├── lib.rs
├── traits.rs
├── handlers/
│   ├── mod.rs
│   ├── handler.rs       ← trait FontHandler
│   ├── bold.rs          ← BoldHandler
│   ├── italic.rs        ← ItalicHandler
│   ├── underline.rs     ← UnderlineHandler
│   ├── strikethrough.rs ← StrikethroughHandler
│   └── surline.rs       ← SurlineHandler
└── nodes/
    ├── mod.rs
    ├── container_node.rs
    ├── inline_node.rs
    ├── list_item_node.rs
    └── text_node.rs
```

---

## Trait `FontHandler`

```rust
// converter/src/handlers/handler.rs

/// Contrat d'un handler de conversion de style Unicode.
///
/// Chaque implémentation gère un style (Bold, Italic, etc.) et expose
/// une seule opération : convertir du texte brut vers le style cible.
pub trait FontHandler: Send + Sync {
    /// Convertit le texte vers le style Unicode cible.
    /// Les espaces et sauts de ligne sont préservés tels quels.
    fn apply(&self, text: &str) -> String;
}
```

---

## Table des accents partagée

Tous les handlers partagent la même table d'accents. Un accent est décomposé en
`<lettre_de_base> + <combining_character>`, puis la lettre de base est stylisée
indépendamment du combining character.

```rust
// converter/src/handlers/accents.rs

/// Décompose un caractère accentué en (lettre_de_base, combining_char).
/// Retourne `None` si le caractère n'a pas de décomposition connue.
pub fn decompose_accent(c: char) -> Option<(char, &'static str)> {
    match c {
        // Français
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
        'ò' => Some(('o', "\u{0300}")),
        'ó' => Some(('o', "\u{0301}")),
        'ô' => Some(('o', "\u{0302}")),
        'ö' => Some(('o', "\u{0308}")),
        'ì' => Some(('i', "\u{0300}")),
        'í' => Some(('i', "\u{0301}")),
        'î' => Some(('i', "\u{0302}")),
        'ï' => Some(('i', "\u{0308}")),
        'ç' => Some(('c', "\u{0327}")),
        // Espagnol
        'ñ' => Some(('n', "\u{0303}")),
        // Portugais
        'ã' => Some(('a', "\u{0303}")),
        'õ' => Some(('o', "\u{0303}")),
        // Turc
        'ğ' => Some(('g', "\u{0306}")),
        'ş' => Some(('s', "\u{0327}")),
        // Polonais
        'ą' => Some(('a', "\u{0328}")),
        'ć' => Some(('c', "\u{0301}")),
        'ę' => Some(('e', "\u{0328}")),
        'ł' => Some(('l', "\u{0337}")),
        'ń' => Some(('n', "\u{0301}")),
        'ś' => Some(('s', "\u{0301}")),
        'ź' => Some(('z', "\u{0301}")),
        'ż' => Some(('z', "\u{0307}")),
        // Majuscules
        'É' => Some(('E', "\u{0301}")),
        'È' => Some(('E', "\u{0300}")),
        'Ê' => Some(('E', "\u{0302}")),
        'Ë' => Some(('E', "\u{0308}")),
        'À' => Some(('A', "\u{0300}")),
        'Á' => Some(('A', "\u{0301}")),
        'Â' => Some(('A', "\u{0302}")),
        'Ä' => Some(('A', "\u{0308}")),
        'Ù' => Some(('U', "\u{0300}")),
        'Ú' => Some(('U', "\u{0301}")),
        'Û' => Some(('U', "\u{0302}")),
        'Ü' => Some(('U', "\u{0308}")),
        'Ò' => Some(('O', "\u{0300}")),
        'Ó' => Some(('O', "\u{0301}")),
        'Ô' => Some(('O', "\u{0302}")),
        'Ö' => Some(('O', "\u{0308}")),
        'Ì' => Some(('I', "\u{0300}")),
        'Í' => Some(('I', "\u{0301}")),
        'Î' => Some(('I', "\u{0302}")),
        'Ï' => Some(('I', "\u{0308}")),
        'Ç' => Some(('C', "\u{0327}")),
        'Ã' => Some(('A', "\u{0303}")),
        'Ñ' => Some(('N', "\u{0303}")),
        'Õ' => Some(('O', "\u{0303}")),
        'Ğ' => Some(('G', "\u{0306}")),
        'İ' => Some(('I', "\u{0307}")),
        'Ş' => Some(('S', "\u{0327}")),
        'Ą' => Some(('A', "\u{0328}")),
        'Ć' => Some(('C', "\u{0301}")),
        'Ę' => Some(('E', "\u{0328}")),
        'Ł' => Some(('L', "\u{0337}")),
        'Ń' => Some(('N', "\u{0301}")),
        'Ś' => Some(('S', "\u{0301}")),
        'Ź' => Some(('Z', "\u{0301}")),
        'Ż' => Some(('Z', "\u{0307}")),
        _ => None,
    }
}
```

---

## `BoldHandler` — exemple d'implémentation complète

```rust
// converter/src/handlers/bold.rs

use std::collections::HashMap;
use once_cell::sync::OnceCell;
use super::{accents::decompose_accent, handler::FontHandler};

const BASE_UPPER: u32 = 0x1D5D4; // 𝗔
const BASE_LOWER: u32 = 0x1D5EE; // 𝗮
const BASE_DIGIT: u32 = 0x1D7EC; // 𝟬

pub struct BoldHandler {
    /// normal → bold unicode (sens unique)
    map: HashMap<String, String>,
}

impl BoldHandler {
    pub fn new() -> Self {
        let mut map = HashMap::new();

        // Lettres A–Z et a–z
        for i in 0u32..26 {
            let upper = char::from_u32('A' as u32 + i).unwrap().to_string();
            let lower = char::from_u32('a' as u32 + i).unwrap().to_string();
            map.insert(upper, char::from_u32(BASE_UPPER + i).unwrap().to_string());
            map.insert(lower, char::from_u32(BASE_LOWER + i).unwrap().to_string());
        }

        // Chiffres 0–9
        for i in 0u32..10 {
            let digit = char::from_u32('0' as u32 + i).unwrap().to_string();
            map.insert(digit, char::from_u32(BASE_DIGIT + i).unwrap().to_string());
        }

        // Accents : lettre_de_base bold + combining character
        for &c in Self::accent_candidates() {
            if let Some((base, combining)) = decompose_accent(c) {
                if let Some(bold_base) = map.get(&base.to_string()) {
                    map.insert(c.to_string(), format!("{}{}", bold_base, combining));
                }
            }
        }

        Self { map }
    }

    /// Segmente le texte en graphèmes.
    /// Utilise `unicode-segmentation` en production pour les séquences multi-codepoints.
    fn graphemes(text: &str) -> impl Iterator<Item = &str> {
        use unicode_segmentation::UnicodeSegmentation;
        text.graphemes(true)
    }

    fn accent_candidates() -> &'static [char] {
        &[
            'é','è','ê','ë','à','á','â','ä','ù','ú','û','ü',
            'ò','ó','ô','ö','ì','í','î','ï','ç','ñ','ã','õ',
            'ğ','ş','ą','ć','ę','ł','ń','ś','ź','ż',
            'É','È','Ê','Ë','À','Á','Â','Ä','Ù','Ú','Û','Ü',
            'Ò','Ó','Ô','Ö','Ì','Í','Î','Ï','Ç','Ã','Ñ','Õ',
            'Ğ','İ','Ş','Ą','Ć','Ę','Ł','Ń','Ś','Ź','Ż',
        ]
    }
}

impl FontHandler for BoldHandler {
    fn apply(&self, text: &str) -> String {
        Self::graphemes(text)
            .map(|g| {
                if g.trim().is_empty() {
                    g.to_string()  // espaces, \n, \t préservés
                } else {
                    self.map.get(g).cloned().unwrap_or_else(|| g.to_string())
                }
            })
            .collect()
    }
}

/// Singleton : la map est construite une seule fois au premier appel.
pub fn bold_handler() -> &'static BoldHandler {
    static INSTANCE: OnceCell<BoldHandler> = OnceCell::new();
    INSTANCE.get_or_init(BoldHandler::new)
}
```

> **Note** : les autres handlers (`ItalicHandler`, `UnderlineHandler`, `StrikethroughHandler`, `SurlineHandler`) suivent exactement le même patron — seuls les codepoints de base changent. `UnderlineHandler` et `StrikethroughHandler` n'ont pas de table de codepoints : ils appliquent directement un combining character (`\u{0332}`, `\u{0336}`) après chaque graphème.

---

## Codepoints de base par style

| Style | Base majuscules | Base minuscules | Base chiffres |
|---|---|---|---|
| `Bold` | `0x1D5D4` | `0x1D5EE` | `0x1D7EC` |
| `Italic` | `0x1D608` | `0x1D622` | — |
| `BoldItalic` | `0x1D63C` | `0x1D656` | — |
| `Underline` | combining `\u{0332}` après chaque char | | |
| `Strikethrough` | combining `\u{0336}` après chaque char | | |
| `Surline` | `0x1D5D4` (SansSerifBold, approximation visuelle) | `0x1D5EE` | `0x1D7EC` |

---

## Trait `ToUnicode` et dispatch

```rust
// converter/src/traits.rs

/// Capacité à se convertir en texte Unicode formaté.
/// Défini et implémenté uniquement dans `converter`.
pub trait ToUnicode {
    fn to_unicode(&self) -> String;
}
```

Le `global_font` disparaît de la signature — c'est maintenant le `ContainerType` qui détermine quel handler appeler :

```rust
// converter/src/nodes/container_node.rs

use parser::ast::{ContainerNode, ContainerType};
use crate::{
    handlers::{
        bold::bold_handler,
        italic::italic_handler,
        underline::underline_handler,
        strikethrough::strikethrough_handler,
        surline::surline_handler,
    },
    nodes::inline_node::inline_to_unicode,
    traits::ToUnicode,
};

impl ToUnicode for ContainerNode {
    fn to_unicode(&self) -> String {
        match &self.container_type {

            ContainerType::Text => {
                let content: String = self.children.iter()
                    .map(|n| inline_to_unicode(n))
                    .collect();
                format!("{}\n", content)
            }

            ContainerType::Bold => {
                let raw: String = self.children.iter()
                    .map(|n| inline_to_unicode(n))
                    .collect();
                bold_handler().apply(&raw)
            }

            ContainerType::Italic => {
                let raw: String = self.children.iter()
                    .map(|n| inline_to_unicode(n))
                    .collect();
                italic_handler().apply(&raw)
            }

            ContainerType::Underline => {
                let raw: String = self.children.iter()
                    .map(|n| inline_to_unicode(n))
                    .collect();
                underline_handler().apply(&raw)
            }

            ContainerType::Strikethrough => {
                let raw: String = self.children.iter()
                    .map(|n| inline_to_unicode(n))
                    .collect();
                strikethrough_handler().apply(&raw)
            }

            ContainerType::Surline => {
                let raw: String = self.children.iter()
                    .map(|n| inline_to_unicode(n))
                    .collect();
                surline_handler().apply(&raw)
            }

            // Heading n'existe pas dans ContainerType : les titres sont rejetés
            // par le parser avant d'atteindre le converter.

            ContainerType::Blockquote => {
                let content: String = self.children.iter()
                    .map(|n| inline_to_unicode(n))
                    .collect();
                format!("❝ {}\n", content)
            }

            ContainerType::List => {
                self.children.iter()
                    .map(|item| format!("• {}", inline_to_unicode(item)))
                    .collect()
            }

            ContainerType::OrderedList => {
                self.children.iter().enumerate()
                    .map(|(i, item)| format!("{}. {}", i + 1, inline_to_unicode(item)))
                    .collect()
            }
        }
    }
}
```

```rust
// converter/src/nodes/inline_node.rs

use parser::ast::InlineNode;
use crate::traits::ToUnicode;

pub fn inline_to_unicode(node: &InlineNode) -> String {
    match node {
        InlineNode::Text(n)      => n.to_unicode(),
        InlineNode::Container(n) => n.to_unicode(),
        InlineNode::ListItem(n)  => n.to_unicode(),
    }
}
```

```rust
// converter/src/nodes/text_node.rs

use parser::ast::TextNode;
use crate::traits::ToUnicode;

impl ToUnicode for TextNode {
    fn to_unicode(&self) -> String {
        // Le TextNode retourne son texte brut.
        // C'est le ContainerNode parent qui applique le handler de style.
        self.text.clone()
    }
}
```

```rust
// converter/src/nodes/list_item_node.rs

use parser::ast::ListItemNode;
use crate::{nodes::inline_node::inline_to_unicode, traits::ToUnicode};

impl ToUnicode for ListItemNode {
    fn to_unicode(&self) -> String {
        self.children.iter().map(|n| inline_to_unicode(n)).collect()
    }
}
```

---

## Point d'entrée public

```rust
// converter/src/lib.rs

pub mod handlers;
pub mod nodes;
pub mod traits;

pub use traits::ToUnicode;

use parser::ast::ContainerNode;

pub fn convert(nodes: &[ContainerNode]) -> String {
    nodes.iter().map(|node| node.to_unicode()).collect()
}
```

---

## `Cargo.toml` du crate `converter`

```toml
[dependencies]
parser       = { path = "../parser" }
once_cell    = "1"
unicode-segmentation = "1"   # pour une segmentation correcte des graphèmes
```

---

## Ajouter un nouveau style

1. Ajouter le variant dans `ContainerType` (`parser/src/ast.rs`).
2. Créer `converter/src/handlers/mon_style.rs` implémentant `FontHandler`.
3. Déclarer le singleton `mon_style_handler()` avec `OnceCell`.
4. Ajouter le cas dans `ContainerNode::to_unicode`.

Aucune modification dans `parser`, `service`, `api` ou `mcp`.

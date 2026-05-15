# Audit — Duplication dans les handlers de conversion

**Sévérité**: 🔴 Critique  
**Crates concernées**: `converter`  
**Fichiers**: `converter/src/handlers/strikethrough.rs`, `underline.rs`, `surline.rs`

---

## Problème

Les handlers `StrikethroughHandler`, `UnderlineHandler` et `SurlineHandler` implémentent exactement le même pattern : itérer sur les graphèmes, sauter les espaces et sauts de ligne, et ajouter un combining mark Unicode spécifique à chaque caractère.

Seul le combining mark change entre les trois.

```rust
// strikethrough.rs
text.graphemes(true)
    .map(|g| {
        if g == " " || g == "\n" {
            g.to_string()
        } else {
            format!("{}\u{0336}", g)  // combining long stroke
        }
    })
    .collect()

// underline.rs — identique, sauf \u{0332}
// surline.rs — identique, sauf \u{0305}
```

Environ 30 lignes dupliquées, et chaque correction/optimisation doit être appliquée trois fois.

---

## Recommandation

Extraire une fonction utilitaire `apply_combining_mark` dans un module partagé :

```rust
// converter/src/handlers/shared.rs
pub(crate) fn apply_combining_mark(text: &str, mark: &str) -> String {
    use unicode_segmentation::UnicodeSegmentation;
    text.graphemes(true)
        .map(|g| {
            if g == " " || g == "\n" {
                g.to_string()
            } else {
                format!("{}{}", g, mark)
            }
        })
        .collect()
}
```

Puis dans chaque handler :

```rust
// strikethrough.rs
impl FontHandler for StrikethroughHandler {
    fn apply(&self, text: &str) -> String {
        apply_combining_mark(text, "\u{0336}")
    }
}
```

Ou, encore plus compact, une macro déclarative :

```rust
macro_rules! combining_handler {
    ($name:ident, $mark:literal) => {
        pub struct $name;
        impl FontHandler for $name {
            fn apply(&self, text: &str) -> String {
                apply_combining_mark(text, $mark)
            }
        }
    };
}

combining_handler!(StrikethroughHandler, "\u{0336}");
combining_handler!(UnderlineHandler,     "\u{0332}");
combining_handler!(SurlineHandler,       "\u{0305}");
```

---

## Impact attendu

- Suppression de ~60 lignes dupliquées
- Toute correction ou optimisation future s'applique en un seul endroit
- Facilite l'ajout de nouveaux styles combining (ex. double underline `\u{0333}`)

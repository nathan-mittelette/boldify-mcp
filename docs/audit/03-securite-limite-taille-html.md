# Audit — Absence de limite de taille pour le parser HTML

**Sévérité**: 🔴 Critique  
**Crate concernée**: `parser`  
**Fichier**: `parser/src/html.rs`, ligne 24 (fonction `parse`)

---

## Problème

Le parser Markdown protège contre les inputs trop grands :

```rust
// parser/src/markdown.rs
const MAX_SIZE: usize = 10 * 1024 * 1024; // 10 MB
if input.len() > MAX_SIZE {
    return Err(ParseError::InputTooLarge { ... });
}
```

Le parser HTML n'a **aucune protection équivalente**. Un appelant peut envoyer :
- Un document HTML de taille arbitraire → itération non bornée
- Des balises infiniment imbriquées `<strong><em><u>...<u><em></strong>` → stack potentiellement profond
- Un flux de `<` invalides → itération au byte sans jamais sortir de la branche tag

Ce vecteur est directement exposé via l'API Lambda et le serveur MCP HTTP, sans authentification mentionnée.

---

## Recommandation

### 1. Ajouter une limite de taille

```rust
// parser/src/html.rs
const MAX_SIZE: usize = 10 * 1024 * 1024; // 10 MB

impl Parser for HtmlParser {
    fn parse(&self, input: &str) -> Result<Vec<ContainerNode>, ParseError> {
        if input.len() > MAX_SIZE {
            return Err(ParseError::InputTooLarge {
                max: MAX_SIZE,
                actual: input.len(),
            });
        }
        // ...
    }
}
```

### 2. Ajouter une limite de profondeur d'imbrication

```rust
const MAX_DEPTH: usize = 64;

// Dans la boucle de parsing, tracker la profondeur du stack
if tag_stack.len() > MAX_DEPTH {
    return Err(ParseError::NestingTooDeep { max: MAX_DEPTH });
}
```

### 3. Variante `ParseError` manquante

Vérifier que `ParseError::InputTooLarge` existe déjà dans `parser/src/error.rs` (défini pour Markdown, peut-être pas pour HTML). Si non, il faut l'ajouter ou rendre la variante commune.

---

## Tests à ajouter

```rust
#[test]
fn rejects_html_above_size_limit() {
    let huge = format!("<p>{}</p>", "a".repeat(11 * 1024 * 1024));
    let result = HtmlParser.parse(&huge);
    assert!(matches!(result, Err(ParseError::InputTooLarge { .. })));
}

#[test]
fn rejects_deeply_nested_html() {
    let deep: String = "<strong>".repeat(100) + "x" + &"</strong>".repeat(100);
    let result = HtmlParser.parse(&deep);
    assert!(matches!(result, Err(ParseError::NestingTooDeep { .. })));
}
```

---

## Impact attendu

- Protection contre les attaques DoS par inputs surdimensionnés
- Comportement cohérent entre les parsers Markdown et HTML
- Facilite le dimensionnement des timeouts Lambda

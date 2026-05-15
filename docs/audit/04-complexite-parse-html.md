# Audit — Complexité cyclomatique de la fonction `parse()` HTML

**Sévérité**: 🟠 Haute  
**Crate concernée**: `parser`  
**Fichier**: `parser/src/html.rs`, lignes 24–164

---

## Problème

La fonction `parse()` du `HtmlParser` fait environ 140 lignes et mélange plusieurs responsabilités distinctes :

1. **Tokenization bas niveau** — lecture caractère par caractère, gestion de `i`
2. **Tracking de position** — `line`, `col`, `byte_offset`
3. **Gestion du stack de balises** — push/pop du contexte d'imbrication
4. **Détection DOCTYPE / commentaires** — cas spéciaux à détecter en tête
5. **Fusion de texte** — accumulation du buffer `current_text`
6. **Validation et erreurs** — construction des `ParseError`

La complexité cyclomatique est supérieure à 15. La fonction est difficile à tester unitairement et à faire évoluer sans risque de régression.

---

## Recommandation

Décomposer en fonctions à responsabilité unique :

### Étape 1 — Extraire un tokenizer

```rust
enum HtmlToken<'a> {
    Text(&'a str),
    OpenTag { name: &'a str, self_closing: bool },
    CloseTag(&'a str),
    Comment,
    Doctype,
}

fn tokenize(input: &str) -> impl Iterator<Item = (HtmlToken<'_>, SourcePosition)> {
    // ...
}
```

### Étape 2 — Séparer le builder d'AST

```rust
fn build_ast(
    tokens: impl Iterator<Item = (HtmlToken<'_>, SourcePosition)>
) -> Result<Vec<ContainerNode>, ParseError> {
    let mut stack: Vec<(ContainerType, SourcePosition, Vec<InlineNode>)> = vec![];
    // ...
}
```

### Étape 3 — Isoler le tracking de position

```rust
struct PositionTracker {
    line: usize,
    col: usize,
    byte_offset: usize,
}

impl PositionTracker {
    fn advance(&mut self, c: char) { ... }
    fn current(&self) -> SourcePosition { ... }
}
```

### Résultat

```rust
impl Parser for HtmlParser {
    fn parse(&self, input: &str) -> Result<Vec<ContainerNode>, ParseError> {
        if input.len() > MAX_SIZE { return Err(...); }
        let tokens = tokenize(input);
        build_ast(tokens)
    }
}
```

---

## Bénéfices

| Avant | Après |
|-------|-------|
| 1 fonction de 140 lignes | 3–4 fonctions de 30–40 lignes |
| Complexité ~15 | Complexité ~4 par fonction |
| Impossible à tester unitairement | Tokenizer testable indépendamment |
| Difficile à débugger | Stack traces lisibles |

---

## Tests unitaires facilitées

Avec un tokenizer séparé, on peut écrire :

```rust
#[test]
fn tokenizes_nested_tags() {
    let tokens: Vec<_> = tokenize("<strong>hello</strong>").collect();
    assert_eq!(tokens[0].0, HtmlToken::OpenTag { name: "strong", self_closing: false });
    assert_eq!(tokens[1].0, HtmlToken::Text("hello"));
    assert_eq!(tokens[2].0, HtmlToken::CloseTag("strong"));
}
```

# Audit — Couverture de tests : cas manquants

**Sévérité**: 🟡 Moyen  
**Crates concernées**: `parser`, `converter`, `api-convert`, `api-syntaxes`, `mcp`

---

## Points positifs existants

- Emoji multicodepoint (`👨‍💻`) testé dans parser Markdown et HTML
- Unicode non-latin (chinois, arabe, cyrillique) couvert
- Accents dans tous les styles de formatage
- Lignes vides multiples testées
- Triple imbrication couverte

---

## Cas manquants

### 1. Profondeur d'imbrication excessive

```rust
// À ajouter dans parser/src/html.rs (tests)
#[test]
fn rejects_deeply_nested_tags() {
    let depth = 100;
    let open: String = "<strong>".repeat(depth);
    let close: String = "</strong>".repeat(depth);
    let input = format!("{}x{}", open, close);
    // Devrait retourner Err(ParseError::NestingTooDeep) une fois la limite ajoutée
    let result = HtmlParser.parse(&input);
    assert!(result.is_err());
}

// Markdown
#[test]
fn rejects_deeply_nested_markdown() {
    let input = "**__~~==".repeat(20) + "x" + &"==~~__**".repeat(20);
    let result = MarkdownParser.parse(&input);
    assert!(result.is_err());
}
```

### 2. Imbrication mal fermée (mismatched nesting)

```rust
#[test]
fn rejects_mismatched_nesting() {
    // <strong> fermé avant <em>
    let result = HtmlParser.parse("<strong><em>texte</strong></em>");
    assert!(matches!(result, Err(ParseError::MismatchedTag { .. })));
}
```

Ce cas est actuellement géré de façon silencieuse (stray closing tag ignoré).

### 3. Inputs limites

```rust
#[test]
fn rejects_html_above_size_limit() {
    let huge = format!("<p>{}</p>", "a".repeat(11 * 1024 * 1024));
    assert!(matches!(HtmlParser.parse(&huge), Err(ParseError::InputTooLarge { .. })));
}

#[test]
fn accepts_empty_string() {
    let result = ContentService::new().convert("", "markdown");
    assert_eq!(result.unwrap(), "");
}

#[test]
fn accepts_only_whitespace() {
    let result = ContentService::new().convert("   \n\t  ", "markdown");
    assert_eq!(result.unwrap(), "");
}
```

### 4. API Lambda — requêtes malformées

```rust
// api-convert/src/main.rs (tests)
#[tokio::test]
async fn rejects_missing_content_field() {
    let body = json!({ "syntax": "markdown" }); // "content" absent
    let response = call_handler(body).await;
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn rejects_unknown_syntax() {
    let body = json!({ "content": "**bold**", "syntax": "rst" });
    let response = call_handler(body).await;
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn rejects_malformed_json() {
    let response = call_handler_raw("not-json").await;
    assert_eq!(response.status(), 400);
}
```

### 5. Concurrence — service thread-safe

```rust
// service/tests/
#[tokio::test]
async fn concurrent_conversions_are_consistent() {
    let svc = Arc::new(ContentService::new());
    let handles: Vec<_> = (0..50).map(|_| {
        let svc = Arc::clone(&svc);
        tokio::spawn(async move {
            svc.convert("**bold**", "markdown").unwrap()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles).await;
    for r in results {
        assert_eq!(r.unwrap(), "𝐛𝐨𝐥𝐝");
    }
}
```

### 6. MCP server — outils inconnus

```rust
// mcp/src/server.rs (tests)
#[tokio::test]
async fn unknown_tool_returns_error() {
    // Appeler un outil qui n'existe pas via le protocole MCP
    // et vérifier que le serveur retourne une erreur propre
    // plutôt que de paniquer
}
```

---

## Résumé des cas à ajouter

| Catégorie | Nombre de tests |
|-----------|----------------|
| Profondeur imbrication | 2 |
| Imbrication mal fermée | 1 |
| Limites de taille | 2 |
| API Lambda malformée | 3 |
| Concurrence | 1 |
| MCP outil inconnu | 1 |
| **Total** | **10** |

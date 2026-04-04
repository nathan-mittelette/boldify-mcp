# Architecture Globale

## Vue d'ensemble

`boldify-mcp` est un outil de conversion de contenu structuré (HTML, Markdown) vers du texte Unicode formaté à l'aide de caractères spéciaux (gras, italique, monospace, etc.). L'outillage est exposé via deux interfaces : une API HTTP et un outil MCP (Model Context Protocol) en ligne de commande.

Le projet est organisé en **workspace Rust** avec cinq crates indépendants, chacun ayant une responsabilité unique et des dépendances strictement orientées dans une seule direction.

---

## Diagramme de dépendance entre crates

```
┌──────────────────────────────────────────────────────────────┐
│                        mcp (binaire)                         │
│              clap • MCP protocol • stdin/stdout              │
└───────────────────────────┬──────────────────────────────────┘
                            │ dépend de
┌───────────────────────────▼──────────────────────────────────┐
│                       api (binaire)                          │
│            lambda_http ou actix-web • serde_json             │
└───────────────────────────┬──────────────────────────────────┘
                            │ dépend de
┌───────────────────────────▼──────────────────────────────────┐
│                     service (bibliothèque)                   │
│               thiserror • orchestration métier               │
└──────────┬────────────────┬─────────────────────────────────-┘
           │ dépend de      │ dépend de
┌──────────▼──────┐  ┌──────▼───────────────────────────────--┐
│    converter    │  │              parser                      │
│  (bibliothèque) │  │          (bibliothèque)                  │
│  trait ToUnicode│  │  trait Parser • types AST • ParseError   │
│  impl par type  ├──►  aucune dépendance interne              │
└─────────────────┘  └─────────────────────────────────────────┘
     dépend de parser (pour les types TextNode)
     parser ne dépend JAMAIS de converter
```

**Règle cardinale** : le flux de dépendances est unidirectionnel. `parser` ne connaît ni `converter`, ni `service`, ni `api`, ni `mcp`. Cette contrainte est vérifiable par `cargo build` : toute import circulaire est une erreur de compilation.

---

## Tableau des crates

| Crate       | Type        | Rôle principal                                                      | Dépendances internes    | Technos clés                             |
|-------------|-------------|----------------------------------------------------------------------|-------------------------|------------------------------------------|
| `parser`    | bibliothèque | Parse HTML/Markdown → `Vec<TextNode>` (AST uniforme)                | aucune                  | `scraper` (HTML), `pulldown-cmark` (MD) |
| `converter` | bibliothèque | Convertit `Vec<TextNode>` → `String` Unicode via trait `ToUnicode`  | `parser`                | implémentations `impl Trait for Type`   |
| `service`   | bibliothèque | Orchestre parser + converter, expose une API métier unifiée          | `parser`, `converter`   | `thiserror`, `Arc`                       |
| `api`       | binaire     | Expose le service via HTTP (Lambda ou Actix)                         | `service`               | `lambda_http` ou `actix-web`, `serde_json`, `tokio` |
| `mcp`       | binaire     | Expose le service via le protocole MCP (CLI)                         | `service`               | `clap`, `serde_json`                     |

---

## Responsabilités par couche

### `parser`
- Définit tous les types de l'AST : `TextNode`, `NodeBase`, `Span`, `NodeMetadata`, les structs de nœuds concrets.
- Définit le trait `Parser` et l'erreur `ParseError`.
- Implémente `MarkdownParser` et `HtmlParser`.
- **Ne contient aucune logique de formatage ou de conversion.**

### `converter`
- Définit le trait `ToUnicode` et l'enum `UnicodeFont` (styles de formatage).
- Implémente `ToUnicode` pour chaque type de nœud concret (`HeadingNode`, `ParagraphNode`, etc.) dans des sous-modules dédiés.
- Dispatch via un `match` sur `TextNode` qui appelle `.to_unicode()` sur chaque variant — rendu possible car `converter` détient toutes les implémentations du trait.
- **N'expose aucun type propre vers `parser`.**

### `service`
- Expose `ContentService` avec les méthodes `convert(syntax, content, style)` et `list_syntaxes()`.
- Gère la sélection du parser selon la syntaxe fournie.
- Propage les erreurs via `ServiceError` (englobant `ParseError`).

### `api`
- Deserialise les requêtes HTTP, appelle `ContentService`, sérialise les réponses.
- Gère les erreurs HTTP (400, 404, 500) de façon uniforme.

### `mcp`
- Parse les arguments CLI avec `clap`.
- Implémente le protocole MCP (entrée/sortie JSON sur stdin/stdout).
- Appelle `ContentService` et formate la sortie.

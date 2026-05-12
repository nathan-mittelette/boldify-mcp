# AGENTS.md

This file provides guidance to AI coding agents (Mistral Vibe, CrewAI, GitHub Copilot, etc.) when working with code in this repository.

## Project Overview

**boldify-mcp** is a Rust MCP (Model Context Protocol) server that converts Markdown or HTML text to Unicode-formatted text (bold, italic, strikethrough, underline, highlight). It runs as an MCP CLI server (stdin/stdout), MCP HTTP server, or AWS Lambda APIs.

## Commands

The `mcp` crate is feature-gated: full-workspace commands must enable exactly one transport feature.

```bash
# Build the whole workspace in the same modes CI uses
cargo build --workspace --features mcp/cli
cargo build --workspace --features mcp/http

# Build only the MCP binary in one transport mode
cargo build -p mcp --features cli
cargo build -p mcp --features http

# Full test suite
cargo test --workspace --features mcp/cli
cargo test --workspace --features mcp/http

# Crate-scoped tests
cargo test -p parser
cargo test -p converter
cargo test -p service
cargo test -p api-convert
cargo test -p api-syntaxes

# Integration tests only
cargo test -p service --test integration

# Run a single test by name
cargo test -p service --test integration markdown_liste_produit_puces
cargo test -p parser --lib markdown::tests::parse_unordered_list_multiple_items
cargo test -p mcp convert_markdown_valide_retourne_texte --features cli

# Lint / formatting
cargo fmt --check
cargo clippy --workspace --all-targets --features mcp/cli -- -D warnings
cargo clippy --workspace --all-targets --features mcp/http -- -D warnings
```

## Architecture

Rust workspace with 6 crates and a strictly unidirectional dependency graph:

```
parser      -> defines AST, parsing traits, parse errors, markdown/html parsers
converter   -> depends on parser AST; renders AST to Unicode text via ToUnicode
service     -> orchestrates parser selection + converter; public library entrypoint
mcp         -> MCP server wrapper around service
api-convert -> Lambda wrapper around service.convert()
api-syntaxes-> Lambda wrapper around service.list_syntaxes()
```

**parser** — Source of truth for supported syntax. Parses Markdown or HTML into `Vec<ContainerNode>` and carries source locations through `Span`, `NodeBase`, and `SourcePosition`. Rejects unsupported constructs (headings, code blocks, tables, citations) with `ParseError::UnsupportedSymbol`. Supported inputs: Markdown (`**bold**`, `*italic*`, `~~strike~~`, `==highlight==`, `-`, numbered lists) and HTML (`<b>`, `<strong>`, `<i>`, `<em>`, `<u>`, `<mark>`, `<s>`, `<del>`, `<ul>`, `<ol>`, `<li>`, `<p>`, `<br>`).

**converter** — Owns all Unicode rendering. Style-specific logic lives in `converter/src/handlers/`, and AST-node dispatch lives in `converter/src/nodes/`. Implements the `ToUnicode` trait on each AST node type. Supports 60+ accented characters across 12+ languages. Pattern: shared AST input, handler-per-style, grapheme-aware processing, shared accent decomposition, and whitespace/emoji preservation.

**service** — `ContentService` is the only business-layer entrypoint. Exposes `convert()` and `list_syntaxes()`. Returns an empty string for blank content. Accepts both `markdown` and `md` for Markdown syntax selection.

**mcp** — MCP server using `rmcp` crate. Exposes two tools: `convert` and `list_syntaxes`. Features are mutually exclusive: `cli` uses stdio transport, `http` uses Axum on port 3000.

**api-convert / api-syntaxes** — AWS Lambda handlers wrapping the service layer. Thin HTTP adapters that deserialize input, call `ContentService`, and serialize JSON responses.

## Key Conventions

- **Unidirectional dependencies** — Keep dependencies unidirectional. Do not move Unicode formatting into `parser`, and do not make lower layers depend on `service`, `mcp`, or the Lambda crates.
- **Parser as source of truth** — Treat parser support lists as canonical. If syntax support changes, update the parser implementation and that parser's `supported_symbols()` output; `service`, `mcp`, and the HTTP APIs should keep delegating instead of hardcoding symbol metadata.
- **Explicit rejection** — Unsupported input is rejected explicitly, not ignored. Markdown headings/tables/code blocks and unsupported HTML tags should continue to surface `ParseError::UnsupportedSymbol`; malformed HTML should use the existing parse error variants.
- **Transport split** — Preserve the `mcp` transport split. `cli` and `http` are mutually exclusive features, and workspace build/test/lint commands should always pick one explicitly.
- **French strings** — Keep user-facing strings and most test names in French to match the existing codebase.
- **Converter pattern** — Follow the existing converter pattern for new styles: shared AST input, handler-per-style, grapheme-aware processing, shared accent decomposition, and whitespace/emoji preservation.
- **Empty content** — `ContentService::convert()` intentionally returns an empty string for blank content and accepts both `markdown` and `md` for Markdown syntax selection.

## Feature Flags

The `mcp` crate requires exactly one transport feature at build time:
- `cli` — stdin/stdout MCP transport
- `http` — Axum HTTP server (adds `axum 0.8` dependency)

Both cannot be active simultaneously (enforced at compile time in `mcp/src/main.rs`).

## Key Files

| File | Role |
|------|------|
| `parser/src/ast.rs` | AST node definitions (`ContainerNode`, `InlineNode`, `TextNode`, `ContainerType`) |
| `parser/src/markdown.rs` | Markdown parser |
| `parser/src/html.rs` | HTML parser |
| `converter/src/traits.rs` | `ToUnicode` trait definition |
| `converter/src/handlers/` | Per-style Unicode transformations |
| `converter/src/nodes/` | Trait implementations per AST node type |
| `service/src/lib.rs` | `ContentService` — public API |
| `mcp/src/server.rs` | MCP tool handler implementations |
| `service/tests/integration.rs` | End-to-end integration tests |

## CI

GitHub Actions (`.github/workflows/build.yml`) runs on push/PR to main: fmt check, clippy, tests, and build — separately for both `cli` and `http` feature modes.

## Agent Guidelines

### When Modifying Code

1. **Follow conventions** — Respect all key conventions listed above (unidirectional dependencies, parser as source of truth, explicit rejection, transport split, French strings, converter pattern)
2. **Match style** — Use existing naming conventions, error handling patterns, and code organization
3. **Add tests** — For any new functionality, add corresponding tests in the appropriate test module. Use French test names to match the existing codebase.
4. **Test thoroughly** — Run crate-scoped tests and integration tests. Test with accented characters and edge cases.

### When Reviewing Code

1. **Check architecture** — Ensure unidirectional dependencies are maintained
2. **Verify parser behavior** — Confirm unsupported constructs (headings, code blocks, tables) are properly rejected with `ParseError::UnsupportedSymbol`
3. **Validate Unicode support** — Test with accented characters from multiple languages and verify grapheme-aware processing
4. **Validate feature isolation** — Confirm feature flags work correctly and don't conflict. Build/test commands should always pick one transport feature explicitly.

### MCP Tool Usage

When interacting with the MCP server, two tools are available:
- `list_syntaxes` — Returns supported formatting symbols for markdown or html
- `convert` — Converts formatted text to Unicode-styled output

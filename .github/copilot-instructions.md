# Copilot Instructions

## Build, test, and lint commands

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

## High-level architecture

This workspace is intentionally layered and should stay that way:

```text
parser      -> defines AST, parsing traits, parse errors, markdown/html parsers
converter   -> depends on parser AST; renders AST to Unicode text via ToUnicode
service     -> orchestrates parser selection + converter; public library entrypoint
mcp         -> MCP server wrapper around service
api-convert -> Lambda wrapper around service.convert()
api-syntaxes-> Lambda wrapper around service.list_syntaxes()
```

- `parser` is the source of truth for supported syntax. It parses Markdown or HTML into `Vec<ContainerNode>` and carries source locations through `Span`, `NodeBase`, and `SourcePosition`.
- `converter` owns all Unicode rendering. Style-specific logic lives in `converter/src/handlers/`, and AST-node dispatch lives in `converter/src/nodes/`.
- `service::ContentService` is the only business-layer entrypoint. External surfaces should call `convert()` or `list_syntaxes()` instead of reaching into parser/converter directly.
- `mcp` exposes two tools, `convert` and `list_syntaxes`. The Lambda binaries are thin HTTP adapters that deserialize input, call `ContentService`, and serialize JSON responses.

## Key conventions

- Keep dependencies unidirectional. Do not move Unicode formatting into `parser`, and do not make lower layers depend on `service`, `mcp`, or the Lambda crates.
- Treat parser support lists as canonical. If syntax support changes, update the parser implementation and that parser's `supported_symbols()` output; `service`, `mcp`, and the HTTP APIs should keep delegating instead of hardcoding symbol metadata.
- Unsupported input is rejected explicitly, not ignored. Markdown headings/tables/code blocks and unsupported HTML tags should continue to surface `ParseError::UnsupportedSymbol`; malformed HTML should use the existing parse error variants.
- Preserve the `mcp` transport split. `cli` and `http` are mutually exclusive features, and workspace build/test/lint commands should always pick one explicitly.
- Keep user-facing strings and most test names in French to match the existing codebase.
- Follow the existing converter pattern for new styles: shared AST input, handler-per-style, grapheme-aware processing, shared accent decomposition, and whitespace/emoji preservation.
- `ContentService::convert()` intentionally returns an empty string for blank content and accepts both `markdown` and `md` for Markdown syntax selection.

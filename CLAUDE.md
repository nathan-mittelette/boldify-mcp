# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**boldify-mcp** is a Rust MCP (Model Context Protocol) server that converts Markdown or HTML text to Unicode-formatted text (bold, italic, strikethrough, underline, highlight). It runs as an MCP CLI server (stdin/stdout), MCP HTTP server, or AWS Lambda APIs.

## Commands

```bash
# Build
cargo build --workspace
cargo build -p mcp --features cli     # CLI mode
cargo build -p mcp --features http    # HTTP mode

# Test
cargo test --workspace

# Lint
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --features mcp/cli -- -D warnings

# Run
cargo run -p mcp --features cli
cargo run -p mcp --features http
```

## Architecture

Rust workspace with 6 crates and a strictly unidirectional dependency graph:

```
mcp (binary, CLI or HTTP) / api-convert / api-syntaxes (Lambda binaries)
  └── service (orchestration library)
        ├── parser (HTML/Markdown → AST)
        └── converter (AST → Unicode text)
```

**parser** — Parses input into an AST. Rejects unsupported constructs (headings, code blocks, tables, citations) with `ParseError::UnsupportedSymbol`. Supported inputs: Markdown (`**bold**`, `*italic*`, `~~strike~~`, `==highlight==`, `-`, numbered lists) and HTML (`<b>`, `<strong>`, `<i>`, `<em>`, `<u>`, `<mark>`, `<s>`, `<del>`, `<ul>`, `<ol>`, `<li>`, `<p>`, `<br>`).

**converter** — Implements the `ToUnicode` trait on each AST node type to produce Unicode output. Font handlers are in `converter/src/handlers/`, node dispatch in `converter/src/nodes/`. Supports 60+ accented characters across 12+ languages.

**service** — Thin orchestration layer exposing `ContentService::convert()` and `ContentService::list_syntaxes()`.

**mcp** — MCP server using `rmcp` crate. Features are mutually exclusive: `cli` uses stdio transport, `http` uses Axum on port 3000. Enabling both causes a compile error.

**api-convert / api-syntaxes** — AWS Lambda handlers wrapping the service layer.

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

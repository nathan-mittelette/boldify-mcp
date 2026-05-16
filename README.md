# boldify-mcp

> Convert Markdown or HTML to Unicode-styled text — bold, italic, strikethrough, underline, highlight — perfect for LinkedIn posts.

**boldify-mcp** is a [Model Context Protocol (MCP)](https://modelcontextprotocol.io) server written in Rust that exposes text formatting tools to any MCP-compatible AI assistant (Claude, Cursor, Zed, VS Code, …). It also ships as an AWS Lambda HTTP API.

[![Build](https://github.com/nathan-mittelette/boldify-mcp/actions/workflows/build.yml/badge.svg)](https://github.com/nathan-mittelette/boldify-mcp/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Quick start

**Local (stdio)** — install the binary and point your client to it:

```bash
curl -fsSL https://raw.githubusercontent.com/nathan-mittelette/boldify-mcp/main/install.sh | bash
```

**Remote (HTTP)** — no install needed, use the hosted endpoint directly:

```
https://api.boldify.net/mcp
```

→ Full setup instructions for every client in the **[Wiki](https://github.com/nathan-mittelette/boldify-mcp/wiki)**

---

## Features

- **MCP server** — plug directly into Claude, Cursor, Zed, VS Code, and any MCP-compatible client
- **HTTP Lambda API** — public REST API at `https://api.boldify.net`
- **Markdown & HTML** input support
- **Unicode formatting** — bold, italic, strikethrough, underline, highlight
- **Accent support** — 60+ accented characters across 12+ languages
- **Fast & lightweight** — pure Rust, zero heavy dependencies

---

## Documentation

| Topic | Link |
|-------|------|
| MCP overview (local vs remote) | [Wiki → MCP Overview](https://github.com/nathan-mittelette/boldify-mcp/wiki/MCP-Overview) |
| Local install via `install.sh` | [Wiki → Local Install](https://github.com/nathan-mittelette/boldify-mcp/wiki/MCP-Local) |
| Remote HTTP endpoint | [Wiki → Remote via HTTP](https://github.com/nathan-mittelette/boldify-mcp/wiki/MCP-Remote) |
| MCP tools reference | [Wiki → Tools](https://github.com/nathan-mittelette/boldify-mcp/wiki/MCP-Tools) |
| MCP resources reference | [Wiki → Resources](https://github.com/nathan-mittelette/boldify-mcp/wiki/MCP-Resources) |
| REST API reference | [Wiki → API](https://github.com/nathan-mittelette/boldify-mcp/wiki/API) |

---

## Architecture

The project is a Rust workspace with six crates:

```
boldify-mcp/
├── parser/         Parse HTML or Markdown into an AST
├── converter/      Convert AST nodes to Unicode text
├── service/        Orchestrate parser + converter
├── api-syntaxes/   AWS Lambda — list supported syntaxes
├── api-convert/    AWS Lambda — perform conversion
└── mcp/            MCP server (CLI or HTTP mode)
```

Dependency flow is strictly unidirectional: `mcp → service → converter → parser`.

---

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [cargo-lambda](https://www.cargo-lambda.info/) (for Lambda builds)

### Build

```bash
cargo build --workspace
cargo build -p mcp --features cli   # MCP CLI mode
cargo build -p mcp --features http  # MCP HTTP mode
```

### Test & lint

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --features mcp/cli -- -D warnings
cargo clippy --workspace --all-targets --features mcp/http -- -D warnings
cargo fmt --check
```

---

## Related

- [boldify](https://github.com/nathan-mittelette/boldify) — The web application at [boldify.net](https://boldify.net)

---

## License

[MIT](LICENSE) © [Nathan Mittelette](https://github.com/nathan-mittelette)

---

## Support

[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?style=flat&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/boldify)

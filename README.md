# boldify-mcp

> Convert Markdown or HTML to Unicode-styled text — bold, italic, strikethrough, underline, highlight — perfect for LinkedIn posts.

**boldify-mcp** is a [Model Context Protocol (MCP)](https://modelcontextprotocol.io) server written in Rust that exposes text formatting tools to any MCP-compatible AI assistant (Claude Desktop, Cursor, etc.). It also ships as an AWS Lambda HTTP API.

[![Build](https://github.com/nathan-mittelette/boldify-mcp/actions/workflows/build.yml/badge.svg)](https://github.com/nathan-mittelette/boldify-mcp/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Features

- **MCP server** — plug directly into Claude Desktop or any MCP-compatible client
- **HTTP Lambda API** — deploy as serverless AWS Lambda functions
- **Markdown & HTML** input support
- **Unicode formatting** — bold, italic, strikethrough, underline, surline (highlight)
- **Accent support** — handles 60+ accented characters across 12+ languages
- **Fast & lightweight** — pure Rust, zero heavy dependencies

## Supported Styles

| Style | Markdown | HTML |
|-------|----------|------|
| Bold | `**text**` | `<b>text</b>` / `<strong>text</strong>` |
| Italic | `*text*` | `<i>text</i>` / `<em>text</em>` |
| Strikethrough | `~~text~~` | `<s>text</s>` / `<del>text</del>` |
| Underline | N/A | `<u>text</u>` |
| Highlight | N/A | `<mark>text</mark>` |

> Headings, tables, code blocks, and other elements without Unicode equivalents are intentionally rejected with a clear error message.

---

## Usage with Claude Desktop

Add the following to your Claude Desktop configuration (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "boldify": {
      "command": "boldify-mcp",
      "args": []
    }
  }
}
```

### Available MCP Tools

| Tool | Description |
|------|-------------|
| `list_syntaxes` | List supported formatting symbols for a given syntax (`markdown` or `html`) |
| `convert` | Convert formatted text to Unicode-styled output |

---

## HTTP API

### `GET /syntaxes?syntax=markdown|html`

Returns the list of supported symbols for the given syntax.

### `POST /convert`

```json
{
  "syntax": "markdown",
  "content": "Hello **world**!"
}
```

**Response:**

```json
{
  "result": "Hello 𝘄𝗼𝗿𝗹𝗱!"
}
```

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

See the [`docs/`](docs/) directory for full architectural documentation.

---

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [cargo-lambda](https://www.cargo-lambda.info/) (for Lambda builds)

### Build

```bash
# Build all crates
cargo build --workspace

# Build MCP server (CLI mode)
cargo build -p mcp --features cli

# Build MCP server (HTTP mode)
cargo build -p mcp --features http
```

### Test

```bash
cargo test --workspace
```

### Lint

```bash
cargo clippy --workspace --all-targets -- -D warnings
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

If you find this project useful, consider supporting it:

[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?style=flat&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/boldify)

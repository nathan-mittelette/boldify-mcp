# Service, API HTTP et MCP CLI

## Couche Service

### Rôle

Le crate `service` est le point d'entrée métier unique. Il orchestre le parser et le converter, expose une interface stable (`ContentService`), et gère toutes les erreurs de façon unifiée.

Les crates `api-syntaxes`, `api-convert` et `mcp` ne connaissent que `service`.

### Chaîne de responsabilité pour `list_syntaxes`

```
Client
  → API / MCP  (passe le type de syntaxe : "markdown" ou "html")
    → Service   (sélectionne le bon parser)
      → Parser  (source de vérité : retourne ses SupportedSymbol)
        → Service (agrège et retourne)
          → API / MCP (sérialise la réponse)
```

Le parser est la seule source de vérité sur ce qu'il supporte. Le service ne duplique pas cette information — il demande au parser concerné.

---

### `ServiceError`

```rust
// service/src/error.rs

use parser::ParseError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Erreur de parsing : {0}")]
    Parse(#[from] ParseError),

    #[error("Syntaxe non supportée : '{0}'. Syntaxes disponibles : markdown, html")]
    UnsupportedSyntax(String),

    #[error("Contenu manquant")]
    EmptyContent,
}
```

---

### `ContentService`

```rust
// service/src/lib.rs

pub mod error;
pub use error::ServiceError;

use converter::convert;
use parser::{HtmlParser, MarkdownParser, Parser, SupportedSymbol};

pub struct ContentService {
    markdown_parser: MarkdownParser,
    html_parser: HtmlParser,
}

impl ContentService {
    pub fn new() -> Self {
        Self {
            markdown_parser: MarkdownParser,
            html_parser: HtmlParser,
        }
    }

    /// Retourne les symboles supportés par le parser de la syntaxe demandée.
    ///
    /// C'est le parser lui-même qui fournit cette information via `supported_symbols()`.
    /// Le service ne connaît pas les détails de chaque syntaxe.
    pub fn list_syntaxes(&self, syntax: &str) -> Result<Vec<SupportedSymbol>, ServiceError> {
        match syntax.to_lowercase().as_str() {
            "markdown" | "md" => Ok(self.markdown_parser.supported_symbols()),
            "html"             => Ok(self.html_parser.supported_symbols()),
            other              => Err(ServiceError::UnsupportedSyntax(other.to_string())),
        }
    }

    /// Convertit `content` depuis la syntaxe `syntax` vers du texte Unicode.
    pub fn convert(
        &self,
        syntax: &str,
        content: &str,
    ) -> Result<String, ServiceError> {
        if content.trim().is_empty() {
            return Ok(String::new());
        }

        let nodes = match syntax.to_lowercase().as_str() {
            "markdown" | "md" => self.markdown_parser.parse(content)?,
            "html"             => self.html_parser.parse(content)?,
            other              => return Err(ServiceError::UnsupportedSyntax(other.to_string())),
        };

        Ok(convert(&nodes))
    }
}

impl Default for ContentService {
    fn default() -> Self { Self::new() }
}
```

---

## API HTTP — Deux Lambdas distinctes

Le projet expose **deux Lambdas indépendantes**, chacune dans son propre binaire :

| Lambda | Binaire | Rôle |
|---|---|---|
| `boldify-syntaxes` | `api-syntaxes` | Liste les symboles supportés pour une syntaxe donnée |
| `boldify-convert` | `api-convert` | Convertit un contenu vers du texte Unicode |

Deux Lambdas séparées permettent des politiques de scaling, de timeout et d'IAM indépendantes. La Lambda `syntaxes` est légère et rapide ; la Lambda `convert` peut traiter des contenus plus lourds avec un timeout plus long.

---

### Lambda `boldify-syntaxes`

**Endpoint** : `GET /syntaxes?syntax=markdown` ou `GET /syntaxes?syntax=html`

```rust
// api-syntaxes/src/main.rs

mod models;

use lambda_http::{run, service_fn, Body, Error, Request, Response};
use models::{ErrorResponse, SyntaxesResponse};
use service::ContentService;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let service = Arc::new(ContentService::new());
    run(service_fn(move |req| {
        let svc = Arc::clone(&service);
        handle(req, svc)
    }))
    .await
}

async fn handle(
    request: Request,
    service: Arc<ContentService>,
) -> Result<Response<Body>, Error> {
    // Extrait le paramètre ?syntax= depuis la query string
    let syntax = request
        .uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find(|p| p.starts_with("syntax="))
                .and_then(|p| p.strip_prefix("syntax="))
        })
        .unwrap_or("")
        .to_string();

    if syntax.is_empty() {
        return json_response(400, &ErrorResponse {
            error: "Paramètre 'syntax' manquant. Valeurs acceptées : markdown, html".to_string(),
        });
    }

    match service.list_syntaxes(&syntax) {
        Ok(symbols) => json_response(200, &SyntaxesResponse { syntax, symbols }),
        Err(e)      => json_response(400, &ErrorResponse { error: e.to_string() }),
    }
}

fn json_response<T: serde::Serialize>(status: u16, body: &T) -> Result<Response<Body>, Error> {
    let json = serde_json::to_string(body)?;
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(json))?)
}
```

```rust
// api-syntaxes/src/models.rs

use parser::SupportedSymbol;
use serde::Serialize;

#[derive(Serialize)]
pub struct SyntaxesResponse {
    pub syntax: String,
    pub symbols: Vec<SupportedSymbol>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
```

**Exemple de réponse** :

```json
GET /syntaxes?syntax=markdown

{
  "syntax": "markdown",
  "symbols": [
    { "symbol": "**", "description": "Gras",     "example": "**texte**" },
    { "symbol": "*",  "description": "Italique",  "example": "*texte*" },
    { "symbol": "~~", "description": "Barré",     "example": "~~texte~~" },
    { "symbol": "==", "description": "Surligné",  "example": "==texte==" },
    { "symbol": "> ", "description": "Citation",  "example": "> texte" },
    { "symbol": "- ", "description": "Liste",     "example": "- item" },
    { "symbol": "1. ","description": "Liste ordonnée", "example": "1. item" }
  ]
}
```

---

### Lambda `boldify-convert`

**Endpoint** : `POST /convert`

```rust
// api-convert/src/main.rs

mod models;

use lambda_http::{run, service_fn, Body, Error, Request, Response};
use models::{ConvertRequest, ConvertResponse, ErrorResponse};
use service::ContentService;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let service = Arc::new(ContentService::new());
    run(service_fn(move |req| {
        let svc = Arc::clone(&service);
        handle(req, svc)
    }))
    .await
}

async fn handle(
    request: Request,
    service: Arc<ContentService>,
) -> Result<Response<Body>, Error> {
    let bytes = request.body().as_ref();
    match serde_json::from_slice::<ConvertRequest>(bytes) {
        Err(e) => json_response(400, &ErrorResponse { error: e.to_string() }),
        Ok(req) => match service.convert(&req.syntax, &req.content) {
            Ok(result) => json_response(200, &ConvertResponse { result }),
            Err(e)     => json_response(400, &ErrorResponse { error: e.to_string() }),
        },
    }
}

fn json_response<T: serde::Serialize>(status: u16, body: &T) -> Result<Response<Body>, Error> {
    let json = serde_json::to_string(body)?;
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(json))?)
}
```

```rust
// api-convert/src/models.rs

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ConvertRequest {
    /// Syntaxe du contenu : `"markdown"` ou `"html"`.
    pub syntax: String,
    /// Contenu à convertir.
    pub content: String,
}

#[derive(Serialize)]
pub struct ConvertResponse {
    pub result: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
```

**Exemple de requête/réponse** :

```json
POST /convert
{ "syntax": "markdown", "content": "**Bonjour** *monde*" }

{ "result": "𝗕𝗼𝗻𝗷𝗼𝘂𝗿 𝘮𝘰𝘯𝘥𝘦" }
```

---

## MCP

### SDK MCP : `rmcp`

Le crate `mcp` utilise [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk) — le SDK Rust officiel du protocole MCP. Il gère automatiquement :

- Le handshake `initialize` / `initialized`
- La sérialisation/désérialisation JSON-RPC
- Le dispatch des méthodes `tools/list` et `tools/call`
- Le transport (stdin/stdout pour CLI, HTTP/SSE pour serveur)

Cela remplace le code JSON-RPC manuel documenté dans les versions précédentes.

```toml
# mcp/Cargo.toml
[dependencies]
rmcp = { version = "0.1", features = ["server", "transport-io", "transport-sse-server"] }
```

### Architecture : features Cargo pour choisir le mode de déploiement

Le crate `mcp` expose une **feature Cargo** qui détermine le mode d'exposition au moment du build :

```toml
# mcp/Cargo.toml

[features]
default  = []
cli      = ["rmcp/transport-io"]           # stdin/stdout — pour les postes locaux
http     = ["rmcp/transport-sse-server",   # HTTP/SSE — pour Lambda ou serveur
            "tokio/full", "dep:axum"]

[dependencies]
rmcp     = { version = "0.1" }
service  = { path = "../service" }
tokio    = { version = "1", optional = true }
axum     = { version = "0.7", optional = true }
serde_json = "1"
```

**Build CLI** (pour les utilisateurs qui tournent le MCP localement) :
```bash
cargo build --release --package mcp --features cli
```

**Build HTTP** (pour un déploiement Lambda ou serveur dédié) :
```bash
cargo build --release --package mcp --features http
```

### Implémentation du serveur MCP avec `rmcp`

```rust
// mcp/src/server.rs

use rmcp::{tool, ServerHandler, ServiceExt, model::*};
use service::ContentService;

/// Serveur MCP exposant les outils list_syntaxes et convert.
#[derive(Clone)]
pub struct BoldifyServer {
    service: std::sync::Arc<ContentService>,
}

impl BoldifyServer {
    pub fn new() -> Self {
        Self { service: std::sync::Arc::new(ContentService::new()) }
    }
}

#[rmcp::tool_router]
impl BoldifyServer {
    /// Liste les symboles supportés pour une syntaxe donnée.
    #[tool(description = "Liste les symboles supportés pour une syntaxe donnée (markdown ou html)")]
    async fn list_syntaxes(
        &self,
        /// Syntaxe cible : `markdown` ou `html`.
        #[tool(param)] syntax: String,
    ) -> Result<CallToolResult, McpError> {
        match self.service.list_syntaxes(&syntax) {
            Ok(symbols) => Ok(CallToolResult::success(vec![
                Content::text(serde_json::to_string(&symbols).unwrap()),
            ])),
            Err(e) => Err(McpError::invalid_params(e.to_string(), None)),
        }
    }

    /// Convertit du contenu HTML ou Markdown en texte Unicode formaté.
    #[tool(description = "Convertit du HTML ou Markdown en texte Unicode formaté")]
    async fn convert(
        &self,
        /// Syntaxe du contenu : `markdown` ou `html`.
        #[tool(param)] syntax: String,
        /// Contenu à convertir.
        #[tool(param)] content: String,
    ) -> Result<CallToolResult, McpError> {
        match self.service.convert(&syntax, &content) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
            Err(e)     => Err(McpError::invalid_params(e.to_string(), None)),
        }
    }
}

impl ServerHandler for BoldifyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "boldify".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            ..Default::default()
        }
    }
}
```

### Point d'entrée conditionnel selon la feature

```rust
// mcp/src/main.rs

mod server;
use server::BoldifyServer;

#[tokio::main]
async fn main() {
    let server = BoldifyServer::new();

    #[cfg(feature = "cli")]
    {
        // Mode CLI : stdin/stdout (pour Claude Desktop, etc.)
        use rmcp::transport::io::stdio;
        server.serve(stdio()).await.expect("Erreur serveur MCP CLI");
    }

    #[cfg(feature = "http")]
    {
        // Mode HTTP/SSE : écoute sur un port (pour Lambda ou serveur dédié)
        use rmcp::transport::sse_server::SseServerTransport;
        let transport = SseServerTransport::new("0.0.0.0:3000");
        server.serve(transport).await.expect("Erreur serveur MCP HTTP");
    }

    #[cfg(not(any(feature = "cli", feature = "http")))]
    compile_error!("Choisissez une feature : --features cli  ou  --features http");
}
```

### Outils exposés

| Outil MCP | Paramètres | Équivalent Lambda |
|---|---|---|
| `list_syntaxes` | `syntax: string` | `boldify-syntaxes` |
| `convert` | `syntax: string`, `content: string` | `boldify-convert` |

> Le protocole JSON-RPC est entièrement géré par `rmcp` — voir `mcp/src/server.rs` ci-dessus. Il n'y a pas de code de dispatch manuel à écrire.

---

## Annexe : `Cargo.toml` workspace

### `Cargo.toml` (racine)

```toml
[workspace]
members = [
    "parser",
    "converter",
    "service",
    "api-syntaxes",
    "api-convert",
    "mcp",
]
resolver = "2"

[profile.release]
lto = true
opt-level = "z"
codegen-units = 1
strip = true
```

### `parser/Cargo.toml`

```toml
[package]
name = "parser"
version = "0.1.0"
edition = "2021"

[dependencies]
thiserror = "1"
serde = { version = "1", features = ["derive"] }  # pour SupportedSymbol
# Pas de dépendance externe : HTML et Markdown sont parsés caractère par caractère.
```

### `converter/Cargo.toml`

```toml
[package]
name = "converter"
version = "0.1.0"
edition = "2021"

[dependencies]
parser      = { path = "../parser" }
once_cell   = "1"
unicode-segmentation = "1"
```

### `service/Cargo.toml`

```toml
[package]
name = "service"
version = "0.1.0"
edition = "2021"

[dependencies]
parser    = { path = "../parser" }
converter = { path = "../converter" }
thiserror = "1"
```

### `api-syntaxes/Cargo.toml`

```toml
[package]
name = "api-syntaxes"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "bootstrap"
path = "src/main.rs"

[dependencies]
service     = { path = "../service" }
lambda_http = "0.12"
tokio       = { version = "1", features = ["full"] }
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
```

### `api-convert/Cargo.toml`

```toml
[package]
name = "api-convert"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "bootstrap"
path = "src/main.rs"

[dependencies]
service     = { path = "../service" }
lambda_http = "0.12"
tokio       = { version = "1", features = ["full"] }
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
```

### `mcp/Cargo.toml`

```toml
[package]
name = "mcp"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "boldify"
path = "src/main.rs"

[dependencies]
service    = { path = "../service" }
clap       = { version = "4", features = ["derive"] }
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

### Déploiement

```bash
# Build des deux Lambdas
cargo lambda build --release --package api-syntaxes
cargo lambda build --release --package api-convert

# Déploiement
cargo lambda deploy --package api-syntaxes --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-role
cargo lambda deploy --package api-convert  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-role

# Build du binaire MCP
cargo build --release --package mcp
```

### Commandes utiles

| Action | Commande |
|---|---|
| Builder le workspace | `cargo build --workspace` |
| Lancer tous les tests | `cargo test --workspace` |
| Vérifier sans compiler | `cargo check --workspace` |
| Builder Lambda syntaxes | `cargo lambda build --release --package api-syntaxes` |
| Builder Lambda convert | `cargo lambda build --release --package api-convert` |
| Builder le binaire MCP | `cargo build --release --package mcp` |
| Générer la documentation | `cargo doc --workspace --open` |

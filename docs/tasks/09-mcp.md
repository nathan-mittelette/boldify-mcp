# Tâche 09 — Crate `mcp` : serveur MCP avec rmcp

## Scope

Implémenter le crate `mcp` exposant deux outils MCP (`convert` et `list_syntaxes`) via la bibliothèque `rmcp`. Deux modes de déploiement sélectionnables via Cargo features : `cli` (stdin/stdout) et `http` (HTTP/SSE). Cette tâche suppose que `service` (tâche 06) est fonctionnel. Les tests **mockent `ContentService`**.

**Référence** : [`05-service-api-mcp.md`](../05-service-api-mcp.md)

---

## Fichiers à créer

```
mcp/src/
├── main.rs    ← point d'entrée, sélection du transport via features
└── server.rs  ← BoldifyServer + impl ServerHandler
```

---

## `mcp/Cargo.toml` — rappel

```toml
[features]
default = []
cli     = ["rmcp/transport-io"]
http    = ["rmcp/transport-sse-server", "tokio/full", "dep:axum"]

[dependencies]
service    = { path = "../service" }
rmcp       = { version = "0.1" }
serde_json = "1"
tokio      = { version = "1", optional = true }
axum       = { version = "0.7", optional = true }
```

---

## `mcp/src/server.rs`

```rust
use rmcp::{ServerHandler, tool, tool_router, model::*, service::RequestContext};
use service::{ContentService, ServiceError};

pub struct BoldifyServer {
    svc: ContentService,
}

impl BoldifyServer {
    pub fn new() -> Self {
        Self { svc: ContentService::new() }
    }
}

#[tool_router]
impl BoldifyServer {
    /// Convertit du contenu HTML ou Markdown en texte Unicode formaté.
    #[tool(description = "Convertit du contenu HTML ou Markdown en texte Unicode formaté (gras, italique, etc.)")]
    async fn convert(
        &self,
        #[tool(param)] syntax:  String,
        #[tool(param)] content: String,
    ) -> String {
        match self.svc.convert(&syntax, &content) {
            Ok(result) => result,
            Err(e)     => format!("Erreur : {}", e),
        }
    }

    /// Liste les symboles supportés par un parser donné.
    #[tool(description = "Liste les symboles/tags supportés pour la syntaxe donnée (markdown ou html)")]
    async fn list_syntaxes(
        &self,
        #[tool(param)] syntax: String,
    ) -> String {
        match self.svc.list_syntaxes(&syntax) {
            Ok(symbols) => serde_json::to_string_pretty(&symbols)
                .unwrap_or_else(|_| "Erreur de sérialisation".to_string()),
            Err(e) => format!("Erreur : {}", e),
        }
    }
}

impl ServerHandler for BoldifyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "boldify".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        }
    }
}
```

---

## `mcp/src/main.rs`

```rust
mod server;
use server::BoldifyServer;

#[cfg(all(feature = "cli", feature = "http"))]
compile_error!("Les features 'cli' et 'http' sont mutuellement exclusives.");

#[cfg(not(any(feature = "cli", feature = "http")))]
compile_error!("Choisissez une feature : --features cli  ou  --features http");

#[cfg(feature = "cli")]
#[tokio::main]
async fn main() {
    use rmcp::ServiceExt;
    use rmcp::transport::stdio;

    let server = BoldifyServer::new();
    server
        .serve(stdio())
        .await
        .expect("Erreur du serveur MCP CLI");
}

#[cfg(feature = "http")]
#[tokio::main]
async fn main() {
    use rmcp::ServiceExt;
    use rmcp::transport::SseServerTransport;

    let server = BoldifyServer::new();
    server
        .serve(SseServerTransport::new("0.0.0.0:3000"))
        .await
        .expect("Erreur du serveur MCP HTTP");
}
```

---

## Tests à implémenter

Fichier : `mcp/src/server.rs` (module `#[cfg(test)]`)

Les tests de cette couche vérifient **uniquement** le comportement des outils MCP (dispatch, format des réponses). Ils utilisent un `ContentService` réel (léger) ou un service mock.

> **Note** : tester le transport (stdin/stdout, HTTP/SSE) n'est pas nécessaire ici — c'est la responsabilité de `rmcp`. On teste uniquement la logique des méthodes `convert` et `list_syntaxes`.

### Stratégie de mock

Comme pour les APIs, extraire les appels service derrière un trait :

```rust
#[cfg(test)]
trait MockableService {
    fn convert(&self, syntax: &str, content: &str) -> Result<String, service::ServiceError>;
    fn list_syntaxes(&self, syntax: &str) -> Result<Vec<parser::SupportedSymbol>, service::ServiceError>;
}
```

### Tests de `convert`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Helper : crée un serveur avec le vrai ContentService
    fn server() -> BoldifyServer { BoldifyServer::new() }
```

```rust
    #[tokio::test]
    async fn convert_markdown_valide_retourne_texte() {
        let s = server();
        let result = s.convert("markdown".to_string(), "texte simple".to_string()).await;
        assert!(!result.is_empty());
        assert!(!result.starts_with("Erreur"));
    }

    #[tokio::test]
    async fn convert_syntaxe_inconnue_retourne_message_erreur() {
        let s = server();
        let result = s.convert("xml".to_string(), "contenu".to_string()).await;
        assert!(result.starts_with("Erreur"));
        assert!(result.contains("xml"));
    }

    #[tokio::test]
    async fn convert_markdown_avec_symbole_non_supporte_retourne_erreur() {
        let s = server();
        let result = s.convert("markdown".to_string(), "# titre".to_string()).await;
        assert!(result.starts_with("Erreur"));
        assert!(result.contains("#"));
    }

    #[tokio::test]
    async fn convert_contenu_vide_retourne_chaine_vide() {
        let s = server();
        let result = s.convert("markdown".to_string(), "".to_string()).await;
        assert_eq!(result, "");
    }
```

### Tests de `list_syntaxes`

```rust
    #[tokio::test]
    async fn list_syntaxes_markdown_retourne_json_valide() {
        let s = server();
        let result = s.list_syntaxes("markdown".to_string()).await;
        // Doit être du JSON parseable
        let parsed: serde_json::Value = serde_json::from_str(&result)
            .expect("La réponse doit être du JSON valide");
        assert!(parsed.is_array());
    }

    #[tokio::test]
    async fn list_syntaxes_html_retourne_json_valide() {
        let s = server();
        let result = s.list_syntaxes("html".to_string()).await;
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array());
        assert!(!parsed.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_syntaxes_syntaxe_inconnue_retourne_message_erreur() {
        let s = server();
        let result = s.list_syntaxes("toml".to_string()).await;
        assert!(result.starts_with("Erreur"));
        assert!(result.contains("toml"));
    }

    #[tokio::test]
    async fn list_syntaxes_markdown_contient_champ_symbol() {
        let s = server();
        let result = s.list_syntaxes("markdown".to_string()).await;
        assert!(result.contains("symbol"));
        assert!(result.contains("description"));
        assert!(result.contains("example"));
    }
```

---

## Compilation feature-gated

Vérifier que le `compile_error!` fonctionne (pas de test automatisé possible, vérification manuelle) :

```bash
# Doit compiler
cargo build --package mcp --features cli
cargo build --package mcp --features http

# Doit échouer à la compilation
cargo build --package mcp  # (sans feature)
```

---

## Critère de succès

```bash
cargo test --package mcp
cargo build --package mcp --features cli
```

Tous les tests passent. `cargo check --workspace` reste vert.

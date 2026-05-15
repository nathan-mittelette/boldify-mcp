# Audit — Absence de logging structuré

**Sévérité**: 🟡 Moyen  
**Crates concernées**: `mcp`, `api-convert`, `api-syntaxes`, `service`  
**Fichiers**: Tous les fichiers `main.rs` et `server.rs`

---

## Problème

Il n'existe **aucun log** dans le code de production. Ni `log!`, `debug!`, `warn!`, `error!`, ni `tracing::info!`. En cas d'erreur en production (Lambda ou MCP server), la seule information disponible est le message d'erreur retourné à l'appelant.

Cela rend impossible :
- Le diagnostic post-mortem d'une erreur Lambda (CloudWatch sans traces applicatives)
- La mesure du temps de traitement par requête
- La détection de patterns d'usage anormaux (inputs très grands, erreurs répétées)
- Le debug du serveur MCP HTTP sans redéployer avec des `println!`

---

## Recommandation

### 1. Ajouter `tracing` comme dépendance

`tracing` est l'écosystème standard Rust pour le logging structuré, compatible avec Lambda, Axum et tokio.

```toml
# Cargo.toml workspace (dependencies partagées)
[workspace.dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

Pour Lambda, ajouter également :
```toml
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

### 2. Initialiser le subscriber au démarrage

```rust
// mcp/src/main.rs et api-convert/src/main.rs
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    // ...
}
```

Pour Lambda (logs JSON → CloudWatch) :
```rust
tracing_subscriber::fmt()
    .json()
    .with_env_filter("info")
    .init();
```

### 3. Instrumenter les points clés

```rust
// service/src/lib.rs
use tracing::{info, warn, instrument};

impl ContentService {
    #[instrument(skip(self), fields(syntax, content_len = content.len()))]
    pub fn convert(&self, content: &str, syntax: &str) -> Result<String, ServiceError> {
        info!("conversion started");
        let result = /* ... */;
        match &result {
            Ok(_) => info!("conversion succeeded"),
            Err(e) => warn!(error = %e, "conversion failed"),
        }
        result
    }
}
```

```rust
// api-convert/src/main.rs
async fn handler(event: Request, svc: &ContentService) -> Response<String> {
    tracing::info!(
        method = %event.method(),
        path = %event.uri().path(),
        "request received"
    );
    // ...
}
```

### 4. Niveaux de log recommandés

| Événement | Niveau |
|-----------|--------|
| Requête reçue | `INFO` |
| Conversion réussie | `DEBUG` |
| Erreur de parsing (input invalide) | `WARN` |
| Erreur interne inattendue | `ERROR` |
| Détail d'un nœud AST | `TRACE` |

---

## Impact attendu

- Diagnostic en production sans redéploiement
- Métriques de latence via CloudWatch Logs Insights
- Détection d'abus (inputs trop grands, erreurs répétées)
- Compatible avec `RUST_LOG=debug` en local pour le debug

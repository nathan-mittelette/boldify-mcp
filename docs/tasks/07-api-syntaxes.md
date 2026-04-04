# Tâche 07 — Lambda `api-syntaxes` : GET /syntaxes

## Scope

Implémenter la Lambda AWS `api-syntaxes` qui expose `GET /syntaxes?syntax=<markdown|html>`. Cette tâche suppose que `service` (tâche 06) est fonctionnel. Les tests **mockent `ContentService`** — ils ne testent pas le parsing ou la conversion.

**Référence** : [`05-service-api-mcp.md`](../05-service-api-mcp.md)

---

## Fichiers à créer

```
api-syntaxes/src/
└── main.rs
```

---

## Comportement attendu

| Requête | Réponse |
|---|---|
| `GET /syntaxes?syntax=markdown` | `200 OK` + JSON array de `SupportedSymbol` |
| `GET /syntaxes?syntax=html` | `200 OK` + JSON array de `SupportedSymbol` |
| `GET /syntaxes?syntax=xml` | `400 Bad Request` + message d'erreur |
| `GET /syntaxes` (sans paramètre) | `400 Bad Request` + "paramètre 'syntax' manquant" |

### Format de réponse 200

```json
[
  { "symbol": "**", "description": "Gras", "example": "**texte gras**" },
  { "symbol": "*",  "description": "Italique", "example": "*texte*" }
]
```

### Format de réponse 400

```json
{ "error": "Syntaxe non supportée : 'xml'. Syntaxes disponibles : markdown, html" }
```

---

## Implémentation

```rust
// api-syntaxes/src/main.rs

use lambda_http::{run, service_fn, Body, Error, Request, Response};
use service::ContentService;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let svc = ContentService::new();
    run(service_fn(|req| handler(req, &svc))).await
}

async fn handler(req: Request, svc: &ContentService) -> Result<Response<Body>, Error> {
    let syntax = extract_query_param(&req, "syntax");

    match syntax {
        None => bad_request("Paramètre 'syntax' manquant. Valeurs acceptées : markdown, html"),
        Some(s) => match svc.list_syntaxes(&s) {
            Ok(symbols) => {
                let json = serde_json::to_string(&symbols)?;
                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(Body::Text(json))?)
            }
            Err(e) => bad_request(&e.to_string()),
        },
    }
}

fn extract_query_param(req: &Request, key: &str) -> Option<String> {
    req.uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find(|p| p.starts_with(&format!("{}=", key)))
                .map(|p| p[key.len() + 1..].to_string())
        })
}

fn bad_request(msg: &str) -> Result<Response<Body>, lambda_http::Error> {
    let body = serde_json::json!({ "error": msg }).to_string();
    Ok(Response::builder()
        .status(400)
        .header("Content-Type", "application/json")
        .body(Body::Text(body))?)
}
```

---

## Tests à implémenter

Fichier : `api-syntaxes/src/main.rs` (module `#[cfg(test)]`)

Les tests de cette couche vérifient **uniquement** la couche HTTP (routing, sérialisation, codes de statut). Ils **mockent `ContentService`** via un trait.

### Stratégie de mock

Extraire la logique dans une fonction prenant un trait `SyntaxesService` :

```rust
trait SyntaxesService {
    fn list_syntaxes(&self, syntax: &str) -> Result<Vec<parser::SupportedSymbol>, service::ServiceError>;
}

impl SyntaxesService for ContentService {
    fn list_syntaxes(&self, syntax: &str) -> Result<Vec<parser::SupportedSymbol>, service::ServiceError> {
        ContentService::list_syntaxes(self, syntax)
    }
}
```

Puis les tests utilisent un `MockSyntaxesService` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use parser::SupportedSymbol;
    use service::ServiceError;

    struct MockSvc {
        result: Result<Vec<SupportedSymbol>, ServiceError>,
    }

    impl SyntaxesService for MockSvc {
        fn list_syntaxes(&self, _: &str) -> Result<Vec<SupportedSymbol>, ServiceError> {
            self.result.clone()
        }
    }

    fn symbols() -> Vec<SupportedSymbol> {
        vec![SupportedSymbol {
            symbol: "**".to_string(),
            description: "Gras".to_string(),
            example: "**x**".to_string(),
        }]
    }
```

### Tests

```rust
    #[tokio::test]
    async fn get_syntaxes_markdown_retourne_200_avec_json() {
        let svc = MockSvc { result: Ok(symbols()) };
        let req = build_request("GET", "/syntaxes?syntax=markdown");
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = body_to_string(resp).await;
        assert!(body.contains("**"));
    }

    #[tokio::test]
    async fn get_syntaxes_sans_param_retourne_400() {
        let svc = MockSvc { result: Ok(symbols()) };
        let req = build_request("GET", "/syntaxes");
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn get_syntaxes_syntaxe_inconnue_retourne_400() {
        let svc = MockSvc {
            result: Err(ServiceError::UnsupportedSyntax("xml".to_string())),
        };
        let req = build_request("GET", "/syntaxes?syntax=xml");
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_to_string(resp).await;
        assert!(body.contains("error"));
    }

    #[tokio::test]
    async fn reponse_200_contient_content_type_json() {
        let svc = MockSvc { result: Ok(symbols()) };
        let req = build_request("GET", "/syntaxes?syntax=markdown");
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[test]
    fn extract_query_param_trouve_le_parametre() {
        // Tester extract_query_param directement (pas besoin d'HTTP)
        // "syntax=markdown" → Some("markdown")
        // "" → None
    }
}
```

---

## Critère de succès

```bash
cargo test --package api-syntaxes
```

Tous les tests passent. `cargo check --workspace` reste vert.

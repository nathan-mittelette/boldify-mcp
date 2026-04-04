# Tâche 08 — Lambda `api-convert` : POST /convert

## Scope

Implémenter la Lambda AWS `api-convert` qui expose `POST /convert`. Cette tâche suppose que `service` (tâche 06) est fonctionnel. Les tests **mockent `ContentService`**.

**Référence** : [`05-service-api-mcp.md`](../05-service-api-mcp.md)

---

## Fichiers à créer

```
api-convert/src/
└── main.rs
```

---

## Comportement attendu

| Requête | Réponse |
|---|---|
| `POST /convert` body `{ "syntax": "markdown", "content": "**gras**" }` | `200 OK` + JSON `{ "result": "..." }` |
| `POST /convert` body JSON invalide | `400 Bad Request` + message d'erreur |
| `POST /convert` body `{ "syntax": "xml", "content": "x" }` | `400 Bad Request` |
| `POST /convert` body `{ "syntax": "markdown", "content": "" }` | `200 OK` + `{ "result": "" }` |
| `POST /convert` body sans champ `syntax` | `400 Bad Request` |

### Format de la requête

```json
{
  "syntax":  "markdown",
  "content": "**Bonjour** *monde*"
}
```

### Format de réponse 200

```json
{ "result": "𝗕𝗼𝗻𝗷𝗼𝘂𝗿 𝘮𝘰𝘯𝘥𝘦" }
```

### Format de réponse 400

```json
{ "error": "Syntaxe non supportée : 'xml'. Syntaxes disponibles : markdown, html" }
```

---

## DTO de la requête

```rust
#[derive(serde::Deserialize)]
struct ConvertRequest {
    syntax:  String,
    content: String,
}
```

---

## Implémentation

```rust
// api-convert/src/main.rs

use lambda_http::{run, service_fn, Body, Error, Request, Response};
use service::ContentService;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let svc = ContentService::new();
    run(service_fn(|req| handler(req, &svc))).await
}

async fn handler(req: Request, svc: &ContentService) -> Result<Response<Body>, Error> {
    let body_bytes = match req.body() {
        Body::Text(s)   => s.as_bytes().to_vec(),
        Body::Binary(b) => b.clone(),
        Body::Empty     => vec![],
    };

    let dto: ConvertRequest = match serde_json::from_slice(&body_bytes) {
        Ok(d)  => d,
        Err(e) => return bad_request(&format!("JSON invalide : {}", e)),
    };

    match svc.convert(&dto.syntax, &dto.content) {
        Ok(result) => {
            let json = serde_json::json!({ "result": result }).to_string();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::Text(json))?)
        }
        Err(e) => bad_request(&e.to_string()),
    }
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

Fichier : `api-convert/src/main.rs` (module `#[cfg(test)]`)

Même stratégie de mock que `api-syntaxes` : extraire la logique dans une fonction prenant un trait `ConvertService`.

```rust
trait ConvertService {
    fn convert(&self, syntax: &str, content: &str) -> Result<String, service::ServiceError>;
}
```

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use service::ServiceError;
    use parser::ParseError;

    struct MockSvc { result: Result<String, ServiceError> }

    impl ConvertService for MockSvc {
        fn convert(&self, _: &str, _: &str) -> Result<String, ServiceError> {
            self.result.clone()
        }
    }
```

```rust
    #[tokio::test]
    async fn post_convert_valide_retourne_200_avec_result() {
        let svc = MockSvc { result: Ok("𝗕𝗼𝗻𝗷𝗼𝘂𝗿".to_string()) };
        let req = build_post_request(r#"{"syntax":"markdown","content":"**Bonjour**"}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = body_to_string(resp).await;
        assert!(body.contains("result"));
        assert!(body.contains("𝗕𝗼𝗻𝗷𝗼𝘂𝗿"));
    }

    #[tokio::test]
    async fn post_convert_json_invalide_retourne_400() {
        let svc = MockSvc { result: Ok("".to_string()) };
        let req = build_post_request("pas du json");
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_to_string(resp).await;
        assert!(body.contains("JSON invalide"));
    }

    #[tokio::test]
    async fn post_convert_syntaxe_inconnue_retourne_400() {
        let svc = MockSvc {
            result: Err(ServiceError::UnsupportedSyntax("xml".to_string())),
        };
        let req = build_post_request(r#"{"syntax":"xml","content":"x"}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn post_convert_erreur_parser_retourne_400() {
        use parser::{ParseError, SourcePosition};
        let svc = MockSvc {
            result: Err(ServiceError::Parse(ParseError::UnsupportedSymbol {
                symbol: "<div>".to_string(),
                position: SourcePosition { line: 1, column: 1, byte_offset: 0 },
            })),
        };
        let req = build_post_request(r#"{"syntax":"html","content":"<div>x</div>"}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_to_string(resp).await;
        assert!(body.contains("<div>"));
    }

    #[tokio::test]
    async fn post_convert_content_vide_retourne_200_avec_chaine_vide() {
        let svc = MockSvc { result: Ok("".to_string()) };
        let req = build_post_request(r#"{"syntax":"markdown","content":""}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = body_to_string(resp).await;
        assert!(body.contains(r#""result":""#));
    }

    #[tokio::test]
    async fn reponse_contient_content_type_json() {
        let svc = MockSvc { result: Ok("x".to_string()) };
        let req = build_post_request(r#"{"syntax":"markdown","content":"x"}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn champ_syntax_manquant_retourne_400() {
        let svc = MockSvc { result: Ok("".to_string()) };
        let req = build_post_request(r#"{"content":"x"}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }
}
```

---

## Critère de succès

```bash
cargo test --package api-convert
```

Tous les tests passent. `cargo check --workspace` reste vert.

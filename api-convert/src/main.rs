use api_shared::{bad_request, body_bytes, ok_json};
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use service::ContentService;
use tracing::{info, warn};

#[derive(serde::Deserialize)]
struct ConvertRequest {
    syntax: String,
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();
    let svc = ContentService::new();
    run(service_fn(|req| handler(req, &svc))).await
}

async fn handler(req: Request, svc: &ContentService) -> Result<Response<Body>, Error> {
    info!(method = %req.method(), path = %req.uri().path(), "request received");
    let bytes = body_bytes(req.body());

    let dto: ConvertRequest = match serde_json::from_slice(&bytes) {
        Ok(d) => d,
        Err(e) => {
            warn!(error = %e, "invalid JSON body");
            return bad_request(&format!("JSON invalide : {}", e));
        }
    };

    match svc.convert(&dto.syntax, &dto.content) {
        Ok(result) => ok_json(serde_json::json!({ "result": result }).to_string()),
        Err(e) => {
            warn!(error = %e, syntax = %dto.syntax, "convert failed");
            bad_request(&e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use api_shared::body_to_string;
    use lambda_http::http::Request as HttpRequest;
    use service::{ContentService, ServiceError};

    trait ConvertService: Send + Sync {
        fn convert(&self, syntax: &str, content: &str) -> Result<String, ServiceError>;
    }

    impl ConvertService for ContentService {
        fn convert(&self, syntax: &str, content: &str) -> Result<String, ServiceError> {
            ContentService::convert(self, syntax, content)
        }
    }

    struct MockSvc {
        result: Result<String, ServiceError>,
    }

    impl ConvertService for MockSvc {
        fn convert(&self, _: &str, _: &str) -> Result<String, ServiceError> {
            self.result.clone()
        }
    }

    async fn handler_with<S: ConvertService>(
        req: Request,
        svc: &S,
    ) -> Result<Response<Body>, Error> {
        let bytes = body_bytes(req.body());

        let dto: ConvertRequest = match serde_json::from_slice(&bytes) {
            Ok(d) => d,
            Err(e) => return bad_request(&format!("JSON invalide : {}", e)),
        };

        match svc.convert(&dto.syntax, &dto.content) {
            Ok(result) => ok_json(serde_json::json!({ "result": result }).to_string()),
            Err(e) => bad_request(&e.to_string()),
        }
    }

    fn build_post_request(body: &str) -> Request {
        HttpRequest::builder()
            .method("POST")
            .uri("/convert")
            .header("Content-Type", "application/json")
            .body(Body::Text(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn post_convert_valide_retourne_200_avec_result() {
        let svc = MockSvc {
            result: Ok("𝗕𝗼𝗻𝗷𝗼𝘂𝗿".to_string()),
        };
        let req = build_post_request(r#"{"syntax":"markdown","content":"**Bonjour**"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = body_to_string(resp.into_body());
        assert!(body.contains("result"));
        assert!(body.contains("𝗕𝗼𝗻𝗷𝗼𝘂𝗿"));
    }

    #[tokio::test]
    async fn post_convert_json_invalide_retourne_400() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request("pas du json");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_to_string(resp.into_body());
        assert!(body.contains("JSON invalide"));
    }

    #[tokio::test]
    async fn post_convert_syntaxe_inconnue_retourne_400() {
        let svc = MockSvc {
            result: Err(ServiceError::UnsupportedSyntax("xml".to_string())),
        };
        let req = build_post_request(r#"{"syntax":"xml","content":"x"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn post_convert_erreur_parser_retourne_400() {
        use parser::{ParseError, SourcePosition};
        let svc = MockSvc {
            result: Err(ServiceError::Parse(ParseError::UnsupportedSymbol {
                symbol: "<div>".to_string(),
                position: SourcePosition {
                    line: 1,
                    column: 1,
                    byte_offset: 0,
                },
            })),
        };
        let req = build_post_request(r#"{"syntax":"html","content":"<div>x</div>"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_to_string(resp.into_body());
        assert!(body.contains("<div>"));
    }

    #[tokio::test]
    async fn post_convert_content_vide_retourne_200_avec_chaine_vide() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request(r#"{"syntax":"markdown","content":""}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = body_to_string(resp.into_body());
        assert!(body.contains(r#""result":""#));
    }

    #[tokio::test]
    async fn reponse_contient_content_type_json() {
        let svc = MockSvc {
            result: Ok("x".to_string()),
        };
        let req = build_post_request(r#"{"syntax":"markdown","content":"x"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn champ_syntax_manquant_retourne_400() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request(r#"{"content":"x"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn champ_content_manquant_retourne_400() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request(r#"{"syntax":"markdown"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn body_json_invalide_contient_message_erreur() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request("{invalid}");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_to_string(resp.into_body());
        assert!(body.contains("error"));
    }

    #[tokio::test]
    async fn reponse_400_contient_content_type_json() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request("pas du json");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }
}

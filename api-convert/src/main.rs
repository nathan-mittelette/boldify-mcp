use lambda_http::{run, service_fn, Body, Error, Request, Response};
use service::ContentService;

#[derive(serde::Deserialize)]
struct ConvertRequest {
    syntax: String,
    content: String,
}

trait ConvertService: Send + Sync {
    fn convert(&self, syntax: &str, content: &str) -> Result<String, service::ServiceError>;
}

impl ConvertService for ContentService {
    fn convert(&self, syntax: &str, content: &str) -> Result<String, service::ServiceError> {
        ContentService::convert(self, syntax, content)
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let svc = ContentService::new();
    run(service_fn(|req| handler(req, &svc))).await
}

async fn handler(req: Request, svc: &ContentService) -> Result<Response<Body>, Error> {
    handler_generic(req, svc).await
}

async fn handler_generic(req: Request, svc: &dyn ConvertService) -> Result<Response<Body>, Error> {
    let body_bytes = match req.body() {
        Body::Text(s) => s.as_bytes().to_vec(),
        Body::Binary(b) => b.clone(),
        Body::Empty => vec![],
        _ => vec![],
    };

    let dto: ConvertRequest = match serde_json::from_slice(&body_bytes) {
        Ok(d) => d,
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

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::http::Request as HttpRequest;
    use service::ServiceError;

    struct MockSvc {
        result: Result<String, ServiceError>,
    }

    impl ConvertService for MockSvc {
        fn convert(&self, _: &str, _: &str) -> Result<String, ServiceError> {
            self.result.clone()
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

    async fn body_to_string(resp: Response<Body>) -> String {
        match resp.into_body() {
            Body::Text(s) => s,
            Body::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
            Body::Empty => String::new(),
            _ => String::new(),
        }
    }

    #[tokio::test]
    async fn post_convert_valide_retourne_200_avec_result() {
        let svc = MockSvc {
            result: Ok("𝗕𝗼𝗻𝗷𝗼𝘂𝗿".to_string()),
        };
        let req = build_post_request(r#"{"syntax":"markdown","content":"**Bonjour**"}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = body_to_string(resp).await;
        assert!(body.contains("result"));
        assert!(body.contains("𝗕𝗼𝗻𝗷𝗼𝘂𝗿"));
    }

    #[tokio::test]
    async fn post_convert_json_invalide_retourne_400() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
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
                position: SourcePosition {
                    line: 1,
                    column: 1,
                    byte_offset: 0,
                },
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
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request(r#"{"syntax":"markdown","content":""}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = body_to_string(resp).await;
        assert!(body.contains(r#""result":""#));
    }

    #[tokio::test]
    async fn reponse_contient_content_type_json() {
        let svc = MockSvc {
            result: Ok("x".to_string()),
        };
        let req = build_post_request(r#"{"syntax":"markdown","content":"x"}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn champ_syntax_manquant_retourne_400() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request(r#"{"content":"x"}"#);
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn body_json_invalide_contient_message_erreur() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request("{invalid}");
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_to_string(resp).await;
        assert!(body.contains("error"));
    }

    #[tokio::test]
    async fn reponse_400_contient_content_type_json() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request("pas du json");
        let resp = handler_generic(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }
}

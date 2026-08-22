use api_shared::{body_bytes, error_response, ok_json};
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use service::{ContentService, ServiceError};
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
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "api-convert starting");
    let svc = ContentService::new();
    run(service_fn(|req| std::future::ready(handler(req, &svc)))).await
}

fn handler(req: Request, svc: &ContentService) -> Result<Response<Body>, Error> {
    info!(method = %req.method(), path = %req.uri().path(), "request received");
    let bytes = body_bytes(req.body());

    let dto: ConvertRequest = match serde_json::from_slice(&bytes) {
        Ok(d) => d,
        Err(e) => {
            warn!(error = %e, "invalid JSON body");
            return error_response("INVALID_JSON", &format!("Invalid JSON: {}", e));
        }
    };

    match svc.convert(&dto.syntax, &dto.content) {
        Ok(result) => ok_json(serde_json::json!({ "result": result }).to_string()),
        Err(e) => {
            warn!(error = %e, syntax = %dto.syntax, "convert failed");
            error_response(service_error_code(&e), &e.to_string())
        }
    }
}

fn service_error_code(e: &ServiceError) -> &'static str {
    use parser::ParseError;
    match e {
        ServiceError::UnsupportedSyntax(_) => "UNSUPPORTED_SYNTAX",
        ServiceError::InputTooLarge { .. } => "INPUT_TOO_LARGE",
        ServiceError::Parse(ParseError::UnsupportedSymbol { .. }) => "UNSUPPORTED_SYMBOL",
        ServiceError::Parse(ParseError::NestingTooDeep { .. }) => "NESTING_TOO_DEEP",
        ServiceError::Parse(ParseError::UnclosedTag { .. }) => "UNCLOSED_TAG",
        ServiceError::Parse(ParseError::InvalidHtml(_)) => "INVALID_HTML",
        ServiceError::Parse(ParseError::InputTooLarge { .. }) => "INPUT_TOO_LARGE",
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
            Err(e) => return error_response("INVALID_JSON", &format!("Invalid JSON: {}", e)),
        };

        match svc.convert(&dto.syntax, &dto.content) {
            Ok(result) => ok_json(serde_json::json!({ "result": result }).to_string()),
            Err(e) => error_response(service_error_code(&e), &e.to_string()),
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
    async fn post_convert_valid_returns_200_with_result() {
        let svc = MockSvc {
            result: Ok("𝗕𝗼𝗹𝗱".to_string()),
        };
        let req = build_post_request(r#"{"syntax":"markdown","content":"**Bold**"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = body_to_string(resp.into_body());
        assert!(body.contains("result"));
        assert!(body.contains("𝗕𝗼𝗹𝗱"));
    }

    #[tokio::test]
    async fn post_convert_invalid_json_returns_400() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request("not json");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_to_string(resp.into_body());
        assert!(body.contains("Invalid JSON"));
    }

    #[tokio::test]
    async fn post_convert_unknown_syntax_returns_400() {
        let svc = MockSvc {
            result: Err(ServiceError::UnsupportedSyntax("xml".to_string())),
        };
        let req = build_post_request(r#"{"syntax":"xml","content":"x"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn post_convert_parser_error_returns_400() {
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
    async fn post_convert_empty_content_returns_200_with_empty_string() {
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
    async fn response_200_contains_content_type_json() {
        let svc = MockSvc {
            result: Ok("x".to_string()),
        };
        let req = build_post_request(r#"{"syntax":"markdown","content":"x"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn missing_syntax_field_returns_400() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request(r#"{"content":"x"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn missing_content_field_returns_400() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request(r#"{"syntax":"markdown"}"#);
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn invalid_json_body_contains_error_message() {
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
    async fn response_400_contains_content_type_json() {
        let svc = MockSvc {
            result: Ok(String::new()),
        };
        let req = build_post_request("not json");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }
}

use api_shared::{bad_request, ok_json};
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
            Ok(symbols) => ok_json(serde_json::to_string(&symbols)?),
            Err(e) => bad_request(&e.to_string()),
        },
    }
}

fn extract_query_param(req: &Request, key: &str) -> Option<String> {
    req.uri().query().and_then(|q| {
        q.split('&')
            .find(|p| p.starts_with(&format!("{}=", key)))
            .map(|p| p[key.len() + 1..].to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use api_shared::body_to_string;
    use lambda_http::http::Request as HttpRequest;
    use parser::SupportedSymbol;
    use service::ServiceError;

    trait SyntaxesService: Send + Sync {
        fn list_syntaxes(&self, syntax: &str) -> Result<Vec<SupportedSymbol>, ServiceError>;
    }

    impl SyntaxesService for ContentService {
        fn list_syntaxes(&self, syntax: &str) -> Result<Vec<SupportedSymbol>, ServiceError> {
            ContentService::list_syntaxes(self, syntax)
        }
    }

    struct MockSvc {
        result: Result<Vec<SupportedSymbol>, ServiceError>,
    }

    impl SyntaxesService for MockSvc {
        fn list_syntaxes(&self, _: &str) -> Result<Vec<SupportedSymbol>, ServiceError> {
            self.result.clone()
        }
    }

    async fn handler_with<S: SyntaxesService>(
        req: Request,
        svc: &S,
    ) -> Result<Response<Body>, Error> {
        let syntax = extract_query_param(&req, "syntax");

        match syntax {
            None => bad_request("Paramètre 'syntax' manquant. Valeurs acceptées : markdown, html"),
            Some(s) => match svc.list_syntaxes(&s) {
                Ok(symbols) => ok_json(serde_json::to_string(&symbols)?),
                Err(e) => bad_request(&e.to_string()),
            },
        }
    }

    fn symbols() -> Vec<SupportedSymbol> {
        vec![SupportedSymbol {
            symbol: "**".to_string(),
            description: "Gras".to_string(),
            example: "**x**".to_string(),
        }]
    }

    fn build_request(method: &str, uri: &str) -> Request {
        HttpRequest::builder()
            .method(method)
            .uri(uri)
            .body(Body::Empty)
            .unwrap()
    }

    #[tokio::test]
    async fn get_syntaxes_markdown_retourne_200_avec_json() {
        let svc = MockSvc {
            result: Ok(symbols()),
        };
        let req = build_request("GET", "/syntaxes?syntax=markdown");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = body_to_string(resp.into_body());
        assert!(body.contains("**"));
    }

    #[tokio::test]
    async fn get_syntaxes_sans_param_retourne_400() {
        let svc = MockSvc {
            result: Ok(symbols()),
        };
        let req = build_request("GET", "/syntaxes");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn get_syntaxes_syntaxe_inconnue_retourne_400() {
        let svc = MockSvc {
            result: Err(ServiceError::UnsupportedSyntax("xml".to_string())),
        };
        let req = build_request("GET", "/syntaxes?syntax=xml");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_to_string(resp.into_body());
        assert!(body.contains("error"));
    }

    #[tokio::test]
    async fn reponse_200_contient_content_type_json() {
        let svc = MockSvc {
            result: Ok(symbols()),
        };
        let req = build_request("GET", "/syntaxes?syntax=markdown");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn reponse_400_contient_content_type_json() {
        let svc = MockSvc {
            result: Ok(symbols()),
        };
        let req = build_request("GET", "/syntaxes");
        let resp = handler_with(req, &svc).await.unwrap();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn body_400_sans_param_contient_message_explicite() {
        let svc = MockSvc {
            result: Ok(symbols()),
        };
        let req = build_request("GET", "/syntaxes");
        let resp = handler_with(req, &svc).await.unwrap();
        let body = body_to_string(resp.into_body());
        assert!(body.contains("syntax"));
    }

    #[test]
    fn extract_query_param_trouve_le_parametre() {
        let req = build_request("GET", "/syntaxes?syntax=markdown");
        assert_eq!(
            extract_query_param(&req, "syntax"),
            Some("markdown".to_string())
        );
    }

    #[test]
    fn extract_query_param_retourne_none_si_absent() {
        let req = build_request("GET", "/syntaxes");
        assert_eq!(extract_query_param(&req, "syntax"), None);
    }

    #[test]
    fn extract_query_param_multiple_params() {
        let req = build_request("GET", "/syntaxes?foo=bar&syntax=html");
        assert_eq!(
            extract_query_param(&req, "syntax"),
            Some("html".to_string())
        );
    }
}

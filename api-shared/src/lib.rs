use lambda_http::{Body, Error, Response};

pub fn bad_request(msg: &str) -> Result<Response<Body>, Error> {
    let body = serde_json::json!({ "error": msg }).to_string();
    Ok(Response::builder()
        .status(400)
        .header("Content-Type", "application/json")
        .body(Body::Text(body))?)
}

pub fn ok_json(body: String) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::Text(body))?)
}

pub fn body_bytes(body: &Body) -> Vec<u8> {
    match body {
        Body::Text(s) => s.as_bytes().to_vec(),
        Body::Binary(b) => b.clone(),
        Body::Empty => vec![],
        _ => vec![],
    }
}

pub fn body_to_string(body: Body) -> String {
    match body {
        Body::Text(s) => s,
        Body::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
        Body::Empty => String::new(),
        _ => String::new(),
    }
}

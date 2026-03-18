use axum::http::{HeaderMap, HeaderValue, Method, Request, StatusCode};
use axum::response::Response;
use futures_util::future::BoxFuture;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct CorsLayer {
    allowed_origins: Arc<Vec<String>>,
}

impl CorsLayer {
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_origins: Arc::new(allowed_origins),
        }
    }
}

impl<S> Layer<S> for CorsLayer {
    type Service = CorsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CorsMiddleware {
            inner,
            allowed_origins: Arc::clone(&self.allowed_origins),
        }
    }
}

#[derive(Clone)]
pub struct CorsMiddleware<S> {
    inner: S,
    allowed_origins: Arc<Vec<String>>,
}

impl<S, ReqBody> Service<Request<ReqBody>> for CorsMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let origin = req
            .headers()
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let request_headers = req
            .headers()
            .get("access-control-request-headers")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let is_preflight = req.method() == Method::OPTIONS;
        let allowed_origins = Arc::clone(&self.allowed_origins);

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let cors_origin = compute_cors_origin(&origin, &allowed_origins);

            if is_preflight {
                let mut response = Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(axum::body::Body::empty())
                    .unwrap();
                inject_cors_headers(response.headers_mut(), &cors_origin, &request_headers);
                return Ok(response);
            }

            let mut response = inner.call(req).await?;
            inject_cors_headers(response.headers_mut(), &cors_origin, &request_headers);
            Ok(response)
        })
    }
}

fn compute_cors_origin(origin: &Option<String>, allowed_origins: &[String]) -> String {
    match origin {
        Some(origin) => {
            if allowed_origins.is_empty() || allowed_origins.contains(origin) {
                origin.clone()
            } else {
                String::new()
            }
        }
        None => "*".to_string(),
    }
}

fn inject_cors_headers(
    headers: &mut HeaderMap,
    cors_origin: &str,
    request_headers: &Option<String>,
) {
    if cors_origin.is_empty() {
        return;
    }

    headers.insert(
        "access-control-allow-origin",
        HeaderValue::from_str(cors_origin).unwrap_or(HeaderValue::from_static("*")),
    );

    headers.insert(
        "access-control-allow-methods",
        HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS"),
    );

    if let Some(req_headers) = request_headers
        && let Ok(val) = HeaderValue::from_str(req_headers)
    {
        headers.insert("access-control-allow-headers", val);
    }

    headers.insert(
        "access-control-expose-headers",
        HeaderValue::from_static("*"),
    );

    if cors_origin != "*" {
        headers.insert(
            "access-control-allow-credentials",
            HeaderValue::from_static("true"),
        );
    }

    headers.insert("access-control-max-age", HeaderValue::from_static("86400"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_origin_with_no_allowlist() {
        let origin = Some("https://example.com".to_string());
        let allowed: Vec<String> = vec![];
        assert_eq!(
            compute_cors_origin(&origin, &allowed),
            "https://example.com"
        );
    }

    #[test]
    fn test_cors_origin_with_matching_allowlist() {
        let origin = Some("https://example.com".to_string());
        let allowed = vec!["https://example.com".to_string()];
        assert_eq!(
            compute_cors_origin(&origin, &allowed),
            "https://example.com"
        );
    }

    #[test]
    fn test_cors_origin_with_non_matching_allowlist() {
        let origin = Some("https://evil.com".to_string());
        let allowed = vec!["https://example.com".to_string()];
        assert_eq!(compute_cors_origin(&origin, &allowed), "");
    }

    #[test]
    fn test_cors_origin_no_origin_header() {
        let origin = None;
        let allowed: Vec<String> = vec![];
        assert_eq!(compute_cors_origin(&origin, &allowed), "*");
    }

    #[test]
    fn test_inject_cors_headers_basic() {
        let mut headers = HeaderMap::new();
        inject_cors_headers(&mut headers, "*", &None);
        assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");
        assert!(headers.get("access-control-allow-credentials").is_none());
    }

    #[test]
    fn test_inject_cors_headers_with_specific_origin() {
        let mut headers = HeaderMap::new();
        inject_cors_headers(&mut headers, "https://example.com", &None);
        assert_eq!(
            headers.get("access-control-allow-origin").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            headers.get("access-control-allow-credentials").unwrap(),
            "true"
        );
    }

    #[test]
    fn test_inject_cors_headers_empty_origin_skips() {
        let mut headers = HeaderMap::new();
        inject_cors_headers(&mut headers, "", &None);
        assert!(headers.get("access-control-allow-origin").is_none());
    }
}

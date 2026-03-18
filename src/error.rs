use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum ProxyError {
    InvalidTargetUrl(String),
    UpstreamRequestFailed(String),
    BodyTooLarge,
    PrivateIpBlocked,
    RateLimited,
    OriginNotAllowed,
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ProxyError::InvalidTargetUrl(reason) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid target URL: {reason}"),
            ),
            ProxyError::UpstreamRequestFailed(reason) => (
                StatusCode::BAD_GATEWAY,
                format!("Upstream request failed: {reason}"),
            ),
            ProxyError::BodyTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "Request body too large".to_string(),
            ),
            ProxyError::PrivateIpBlocked => (
                StatusCode::FORBIDDEN,
                "Requests to private IPs are blocked".to_string(),
            ),
            ProxyError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
            ),
            ProxyError::OriginNotAllowed => {
                (StatusCode::FORBIDDEN, "Origin not allowed".to_string())
            }
        };

        let body = json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}

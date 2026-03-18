use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::Response;
use bytes::Bytes;
use url::Url;

use crate::error::ProxyError;
use crate::security;

const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

#[derive(Clone)]
pub struct AppState {
    pub client: reqwest::Client,
    pub config: crate::config::Config,
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: axum::http::Method,
    uri: axum::http::Uri,
    body: Bytes,
) -> Result<Response, ProxyError> {
    let target_url = extract_target_url(&uri)?;

    if state.config.block_private_ips
        && let Some(host) = target_url.host_str()
    {
        security::validate_target_ip(host)?;
    }

    if body.len() > state.config.max_body_size {
        return Err(ProxyError::BodyTooLarge);
    }

    let upstream_response =
        forward_request(&state.client, &method, &headers, &target_url, body).await?;

    build_response(upstream_response).await
}

fn extract_target_url(uri: &axum::http::Uri) -> Result<Url, ProxyError> {
    let path = uri.path();
    let raw_url = path.strip_prefix('/').unwrap_or(path);

    let full_url = match uri.query() {
        Some(query) => format!("{raw_url}?{query}"),
        None => raw_url.to_string(),
    };

    if full_url.is_empty() {
        return Err(ProxyError::InvalidTargetUrl(
            "No target URL provided. Usage: /<url>".to_string(),
        ));
    }

    let url = Url::parse(&full_url)
        .map_err(|e| ProxyError::InvalidTargetUrl(format!("Failed to parse URL: {e}")))?;

    match url.scheme() {
        "http" | "https" => Ok(url),
        scheme => Err(ProxyError::InvalidTargetUrl(format!(
            "Unsupported scheme: {scheme}"
        ))),
    }
}

async fn forward_request(
    client: &reqwest::Client,
    method: &axum::http::Method,
    headers: &HeaderMap,
    target_url: &Url,
    body: Bytes,
) -> Result<reqwest::Response, ProxyError> {
    let method = convert_method(method);
    let mut req_builder = client.request(method, target_url.as_str());

    for (name, value) in headers {
        let name_str = name.as_str().to_lowercase();
        if name_str == "host" || HOP_BY_HOP_HEADERS.contains(&name_str.as_str()) {
            continue;
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            req_builder = req_builder.header(name.as_str(), v);
        }
    }

    if !body.is_empty() {
        req_builder = req_builder.body(body);
    }

    req_builder
        .send()
        .await
        .map_err(|e| ProxyError::UpstreamRequestFailed(e.to_string()))
}

fn convert_method(method: &axum::http::Method) -> reqwest::Method {
    match *method {
        axum::http::Method::GET => reqwest::Method::GET,
        axum::http::Method::POST => reqwest::Method::POST,
        axum::http::Method::PUT => reqwest::Method::PUT,
        axum::http::Method::DELETE => reqwest::Method::DELETE,
        axum::http::Method::PATCH => reqwest::Method::PATCH,
        axum::http::Method::HEAD => reqwest::Method::HEAD,
        axum::http::Method::OPTIONS => reqwest::Method::OPTIONS,
        _ => reqwest::Method::GET,
    }
}

async fn build_response(upstream: reqwest::Response) -> Result<Response, ProxyError> {
    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

    let mut headers = HeaderMap::new();
    for (name, value) in upstream.headers() {
        let name_str = name.as_str().to_lowercase();
        if HOP_BY_HOP_HEADERS.contains(&name_str.as_str()) {
            continue;
        }
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            headers.append(n, v);
        }
    }

    let body_bytes = upstream.bytes().await.map_err(|e| {
        ProxyError::UpstreamRequestFailed(format!("Failed to read upstream body: {e}"))
    })?;

    let mut response = Response::new(Body::from(body_bytes));
    *response.status_mut() = status;
    *response.headers_mut() = headers;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_target_url_valid() {
        let uri: axum::http::Uri = "/https://example.com/api/data".parse().unwrap();
        let url = extract_target_url(&uri).unwrap();
        assert_eq!(url.as_str(), "https://example.com/api/data");
    }

    #[test]
    fn test_extract_target_url_with_query() {
        let uri: axum::http::Uri = "/https://example.com/api?key=value&foo=bar"
            .parse()
            .unwrap();
        let url = extract_target_url(&uri).unwrap();
        assert_eq!(url.as_str(), "https://example.com/api?key=value&foo=bar");
    }

    #[test]
    fn test_extract_target_url_empty() {
        let uri: axum::http::Uri = "/".parse().unwrap();
        let result = extract_target_url(&uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_target_url_invalid_scheme() {
        let uri: axum::http::Uri = "/ftp://example.com/file".parse().unwrap();
        let result = extract_target_url(&uri);
        assert!(result.is_err());
    }
}

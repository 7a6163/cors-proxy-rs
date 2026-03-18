# cors-proxy-rs

A lightweight, high-performance CORS Anywhere proxy built with Rust. It forwards HTTP requests to any target URL and injects CORS headers into responses, enabling frontend applications to access APIs that lack proper CORS support.

## Features

- Proxy any HTTP/HTTPS request with automatic CORS header injection
- OPTIONS preflight handling (returns 204 without hitting upstream)
- Origin allowlist for restricting which frontends can use the proxy
- Per-IP rate limiting (token bucket via `governor`)
- Private IP blocking to prevent SSRF attacks
- Configurable via CLI args or environment variables
- Request body size limits

## Quick Start

### From source

```bash
cargo run

# Custom port
cargo run -- --port 8080
```

### Docker

```bash
docker build -t cors-proxy-rs .
docker run -p 3000:3000 cors-proxy-rs
```

### Usage

Prepend the proxy URL to your target:

```bash
curl http://localhost:3000/https://httpbin.org/get
```

From JavaScript:

```js
const resp = await fetch('http://localhost:3000/https://api.example.com/data');
const data = await resp.json();
```

## Configuration

All options can be set via CLI flags or environment variables.

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `-p, --port` | `PORT` | `3000` | Port to listen on |
| `--rate-limit-per-minute` | `RATE_LIMIT` | `60` | Max requests per minute per IP |
| `--allowed-origins` | `ALLOWED_ORIGINS` | _(all)_ | Comma-separated origin allowlist |
| `--max-body-size` | `MAX_BODY_SIZE` | `10485760` | Max request body size in bytes |
| `--block-private-ips` | `BLOCK_PRIVATE_IPS` | `true` | Block requests to private/loopback IPs |
| `--timeout-secs` | `TIMEOUT_SECS` | `30` | Upstream request timeout in seconds |

### Examples

```bash
# Restrict to specific origins
cors-proxy-rs --allowed-origins "https://myapp.com,https://staging.myapp.com"

# Higher rate limit, custom port
cors-proxy-rs --port 8080 --rate-limit-per-minute 120

# Using environment variables
PORT=8080 ALLOWED_ORIGINS="https://myapp.com" cors-proxy-rs
```

### Docker with configuration

```bash
docker run -p 8080:8080 \
  -e PORT=8080 \
  -e RATE_LIMIT=120 \
  -e ALLOWED_ORIGINS="https://myapp.com,https://staging.myapp.com" \
  cors-proxy-rs
```

## Security

- **SSRF Protection**: Requests to private IPs (`127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `::1`, `fc00::/7`) are blocked by default
- **Rate Limiting**: Per-IP token bucket prevents abuse
- **Origin Allowlist**: Restrict which frontends can use the proxy
- **Body Size Limits**: Prevents memory exhaustion from oversized payloads
- **Hop-by-hop Header Filtering**: Strips `Connection`, `Keep-Alive`, `Transfer-Encoding`, `Proxy-Authorization`, etc.

## CORS Headers

The proxy injects the following headers on every response:

| Header | Value |
|--------|-------|
| `Access-Control-Allow-Origin` | Echoed from request `Origin` (or `*` if absent) |
| `Access-Control-Allow-Methods` | `GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS` |
| `Access-Control-Allow-Headers` | Echoed from `Access-Control-Request-Headers` |
| `Access-Control-Expose-Headers` | `*` |
| `Access-Control-Allow-Credentials` | `true` (only when origin is not `*`) |
| `Access-Control-Max-Age` | `86400` |

## Development

```bash
# Run tests
cargo test

# Run with clippy
cargo clippy --all-targets --all-features

# Format
cargo fmt
```

## License

MIT

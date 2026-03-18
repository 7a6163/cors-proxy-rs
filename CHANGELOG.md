# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-03-18

### Added

- Docker image auto-publish to GHCR on git tag (`release.yml`)
- Multi-platform Docker support (`linux/amd64`, `linux/arm64`)
- Codecov integration with `cargo-llvm-cov` coverage reports
- Secret audit in CI (TruffleHog + Gitleaks)
- CHANGELOG.md

### Changed

- CI test job now generates LCOV coverage and uploads to Codecov

## [1.0.0] - 2026-03-18

### Added

- Core CORS proxy: forward HTTP/HTTPS requests with automatic CORS header injection
- OPTIONS preflight handling (returns 204 without hitting upstream)
- Origin allowlist (`--allowed-origins`) to restrict which frontends can use the proxy
- Per-IP rate limiting via `governor` token bucket (`--rate-limit-per-minute`)
- Private IP blocking to prevent SSRF (`--block-private-ips`)
- Request body size limits (`--max-body-size`)
- Hop-by-hop header filtering
- Configuration via CLI args (clap) and environment variables
- Dockerfile with multi-stage build (`rust:1.94-slim` + `debian:bookworm-slim`)
- CI pipeline: `cargo fmt`, `cargo clippy`, `cargo test`
- 16 unit tests + 8 integration tests
- README with usage, configuration, and security documentation

[1.1.0]: https://github.com/7a6163/cors-proxy-rs/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/7a6163/cors-proxy-rs/releases/tag/v1.0.0

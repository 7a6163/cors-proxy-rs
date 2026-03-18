FROM rust:1.94-slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN cargo build --release && rm -rf src

COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /app/target/release/cors-proxy-rs /usr/local/bin/cors-proxy-rs

ENV PORT=3000
EXPOSE 3000

ENTRYPOINT ["cors-proxy-rs"]

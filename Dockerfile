# ---- Dashboard build stage ----
FROM node:22-slim AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web ./
RUN npm run build

# ---- API build stage ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# Cache dependencies: build them once with a dummy main, then rebuild only our code.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && touch src/lib.rs \
    && cargo build --release && rm -rf src

COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs src/lib.rs && cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 snip && mkdir /data && chown snip /data

COPY --from=builder /app/target/release/snip-api /usr/local/bin/snip-api
COPY --from=web /web/dist /app/web

USER snip
ENV DATABASE_URL=sqlite:///data/snip.db \
    STATIC_DIR=/app/web \
    PORT=8080 \
    RUST_LOG=snip_api=info,tower_http=info
EXPOSE 8080
CMD ["snip-api"]

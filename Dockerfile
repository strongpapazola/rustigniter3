# syntax=docker/dockerfile:1

# ---------- build stage ----------
FROM rust:1-slim-bookworm AS builder

WORKDIR /app

# Native tools needed by bundled sqlite / crypto crates.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config build-essential ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better layer caching.
COPY Cargo.toml Cargo.lock ./

# Copy source/config/docs used at build time.
COPY src ./src
COPY config ./config
COPY public ./public
COPY user_guide ./user_guide
COPY README.md LICENSE ./

RUN cargo build --release

# ---------- runtime stage ----------
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin rustigniter

COPY --from=builder /app/target/release/rustigniter /usr/local/bin/rustigniter
COPY --from=builder /app/config ./config
COPY --from=builder /app/src/app/views ./src/app/views
COPY --from=builder /app/public ./public
COPY --from=builder /app/user_guide ./user_guide
COPY --from=builder /app/README.md /app/LICENSE ./

# Runtime-writable folders for sqlite DB, sessions, logs, uploads.
RUN mkdir -p /app/storage /app/public/uploads \
    && chown -R rustigniter:rustigniter /app/storage /app/public/uploads

USER rustigniter

EXPOSE 8099

# Default command: apply migrations automatically via serve, then listen on 0.0.0.0:8099
CMD ["rustigniter", "serve"]

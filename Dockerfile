# ---------- Build stage ----------
FROM rust:1.90-bookworm AS builder
WORKDIR /app

ENV RUSTFLAGS="--cfg tokio_unstable"

# System deps for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# ---- Dependency caching ----
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# ---- Build actual app ----
COPY . .
RUN cargo build --release --features aws-s3     
# Adding the aws-s3 feature for using aws s3 storage instead of minio

# ---------- Runtime stage ----------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Non-root user
RUN useradd -m -u 1000 appuser

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/publish3-backend /usr/local/bin/publish3-backend

# Copy migrations (if used at runtime)
COPY --from=builder /app/migrations /app/migrations

# Optional: SQLx offline
# COPY --from=builder /app/sqlx-data.json /app/sqlx-data.json

USER appuser

EXPOSE 8080

ENV RUST_LOG=info

ENTRYPOINT ["publish3-backend"]

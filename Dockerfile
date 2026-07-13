FROM rust:1.78-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    pkg-config \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tools ./tools

RUN cargo build --release --package axiom-api

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/axiom-api /app/axiom-api

RUN mkdir -p /data

ENV RUST_LOG=info
ENV AXIOM_API_PORT=9092
ENV AXIOM_DB_PATH=/data/axiom.sqlite

EXPOSE 9092

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:9092/api/v1/health || exit 1

CMD ["./axiom-api"]
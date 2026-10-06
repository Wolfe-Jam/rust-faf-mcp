# Rust 1.88 is the floor (rust-version in Cargo.toml; rmcp 3 requires it).
FROM rust:1.88-slim AS builder

# reqwest's default TLS is native-tls, which links OpenSSL.
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
# src/skills.rs embeds skills/faf-context/SKILL.md at compile time.
COPY skills ./skills

RUN cargo build --release --locked

FROM debian:bookworm-slim

# OpenSSL at runtime, and CA roots so faf_git can reach the GitHub API.
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rust-faf-mcp /usr/local/bin/rust-faf-mcp

CMD ["rust-faf-mcp"]

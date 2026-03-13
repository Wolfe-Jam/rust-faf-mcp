FROM rust:1.82-slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/rust-faf-mcp /usr/local/bin/rust-faf-mcp

CMD ["rust-faf-mcp"]

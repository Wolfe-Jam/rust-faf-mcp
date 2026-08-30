//! rust-faf-mcp — Rust MCP server for FAF
//!
//! Cart of FAFb (`xai-faf-rust`). 11 tools. Author is the Rust CLI; this MCP consumes.
//! stdio JSON-RPC via rmcp, powered by faf-rust-sdk

use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

mod agents;
mod app_type;
mod inject;
mod intent;
mod interview;
mod server;
mod skills;
mod tools;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!(
        "rust-faf-mcp v{} — MCP Server Starting...",
        env!("CARGO_PKG_VERSION")
    );

    let service = server::FafServer::new()
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?;

    service.waiting().await?;
    Ok(())
}

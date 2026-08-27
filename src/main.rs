//! rust-faf-mcp — Rust MCP server for FAF
//!
//! 10 tools: faf_init, faf_git, faf_read, faf_score, faf_sync, faf_compress, faf_discover, faf_tokens, faf_auto, faf_agents
//! stdio JSON-RPC via rmcp, powered by faf-rust-sdk

use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

mod agents;
mod inject;
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

//! rust-faf-mcp — Rust MCP server for FAF
//!
//! Cart of FAFb (`xai-faf-rust`). 12 tools. Author is the Rust CLI; this MCP consumes.
//! stdio JSON-RPC via rmcp, powered by faf-rust-sdk

use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

mod agents;
mod app_type;
mod dna;
mod inject;
mod intent;
mod interview;
mod server;
mod setup;
mod skills;
mod tools;

const HELP: &str = "\
rust-faf-mcp — Rust-native MCP server for FAF (project.faf, IANA application/vnd.faf+yaml)

Usage:
  rust-faf-mcp              Serve MCP over stdio (what MCP clients run)
  rust-faf-mcp --version    Print the version (-V)
  rust-faf-mcp --help       Print this help (-h)

Run on its own, it waits silently for an MCP client. That's correct; Ctrl+C to quit.

Add it to a client:
  claude mcp add faf -- rust-faf-mcp
  { \"mcpServers\": { \"faf\": { \"command\": \"rust-faf-mcp\" } } }

https://github.com/Wolfe-Jam/rust-faf-mcp";

/// `--version` / `--help` answer and exit before anything else runs, so stdout
/// carries only the answer. Any other argument starts the server as before.
fn handle_flags() -> Option<i32> {
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => {
            println!("rust-faf-mcp {}", env!("CARGO_PKG_VERSION"));
            Some(0)
        }
        Some("--help" | "-h") => {
            println!("{HELP}");
            Some(0)
        }
        _ => None,
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    if let Some(code) = handle_flags() {
        std::process::exit(code);
    }

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

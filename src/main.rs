use anyhow::Result;
use rmcp::ServiceExt;
use rmcp::transport::io::stdio;

use lsq_mcp::{login, server};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("configure") => cmd_configure().await,
        Some("status") => {
            cmd_status();
            Ok(())
        }
        Some(cmd) => {
            eprintln!("Unknown command: {}", cmd);
            eprintln!("Usage: lsq-mcp [configure|status]");
            eprintln!("  (no args)   Start MCP server");
            eprintln!("  configure   Set up your LSQ API keys");
            eprintln!("  status      Show current configuration");
            std::process::exit(1);
        }
        None => cmd_serve().await,
    }
}

async fn cmd_serve() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting lsq-mcp v{}", lsq_mcp::config::VERSION);

    let mcp_server = server::LsqMcpServer::new();
    let service = mcp_server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

async fn cmd_configure() -> Result<()> {
    login::configure().await.map_err(|e| anyhow::anyhow!("{}", e))
}

fn cmd_status() {
    login::status();
}

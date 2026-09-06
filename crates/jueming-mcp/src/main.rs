use jueming_mcp::{McpServer, SidecarConfig, smoke};
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = SidecarConfig::from_environment()?;
    if std::env::args()
        .skip(1)
        .any(|argument| argument == "--smoke")
    {
        smoke(config).await?;
        // Stdio's stdout is still protocol-reserved even in this diagnostic mode.
        eprintln!("Jueming MCP bridge smoke check passed.");
        return Ok(());
    }
    let server = McpServer::new(config)?;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

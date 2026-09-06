[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$workspace = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path.Replace([char]92, [char]47)
$probeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("jueming-agent-qa-" + [guid]::NewGuid().ToString("N"))

New-Item -ItemType Directory -Path (Join-Path $probeRoot "src") | Out-Null
try {
  @"
[package]
name = "jueming-agent-http-qa"
version = "0.0.0"
edition = "2024"

[workspace]

[dependencies]
jueming-application = { path = "$workspace/crates/jueming-application" }
jueming-agent-transport = { path = "$workspace/crates/jueming-agent-transport" }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "net", "io-util"] }
"@ | Set-Content -NoNewline -LiteralPath (Join-Path $probeRoot "Cargo.toml")

  @'
use std::{collections::BTreeSet, sync::Arc, time::Duration};

use jueming_agent_transport::{start_server, AuthToken, ServerConfig};
use jueming_application::{AgentCall, LocalAppHost};
use serde_json::{json, to_string};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

async fn post(address: std::net::SocketAddr, token: Option<&str>, call: AgentCall) -> String {
    let body = to_string(&call).expect("serialize call");
    let authorization = token.map(|token| format!("Authorization: Bearer {token}\r\n")).unwrap_or_default();
    let request = format!(
        "POST /v1/agent/call HTTP/1.1\r\nHost: {address}\r\n{authorization}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len(),
    );
    let mut socket = TcpStream::connect(address).await.expect("connect loopback bridge");
    socket.write_all(request.as_bytes()).await.expect("write request");
    let mut response = String::new();
    socket.read_to_string(&mut response).await.expect("read response");
    response
}

#[tokio::main]
async fn main() {
    let host = Arc::new(LocalAppHost::new());
    let token = AuthToken::new("qa-local-token").expect("token");
    let config = ServerConfig {
        enabled: true,
        bind_addr: "127.0.0.1:0".parse().expect("loopback address"),
        auth_token: token,
        allowed_origins: BTreeSet::new(),
        max_request_bytes: 64 * 1024,
        request_timeout: Duration::from_secs(2),
    };
    let server = start_server(host, config).await.expect("start loopback server");
    let address = server.status().bound_addr.expect("bound address");

    let description = post(address, Some("qa-local-token"), AgentCall {
        request_id: "qa-describe".into(), method: "app.describe".into(), params: json!({}).as_object().unwrap().clone(), binding_id: None,
    }).await;
    assert!(description.starts_with("HTTP/1.1 200"), "describe response: {description}");
    assert!(description.contains("native_ui_only"), "describe response: {description}");

    let unauthorized = post(address, None, AgentCall {
        request_id: "qa-unauthorized".into(), method: "app.describe".into(), params: json!({}).as_object().unwrap().clone(), binding_id: None,
    }).await;
    assert!(unauthorized.starts_with("HTTP/1.1 401"), "unauthorized response: {unauthorized}");

    let approval = post(address, Some("qa-local-token"), AgentCall {
        request_id: "qa-approval".into(), method: "proposal.approve".into(), params: json!({ "proposal_id": "018f7b1a-2c40-7e33-9a11-1e3a98d0f021" }).as_object().unwrap().clone(), binding_id: None,
    }).await;
    assert!(approval.starts_with("HTTP/1.1 403"), "approval response: {approval}");
    assert!(approval.contains("native_only_method"), "approval response: {approval}");

    server.stop().await.expect("stop server");
    println!("transport HTTP auth, dispatch, and native-approval boundary: passed");
}
'@ | Set-Content -NoNewline -LiteralPath (Join-Path $probeRoot "src\main.rs")

  cargo run --quiet --manifest-path (Join-Path $probeRoot "Cargo.toml")
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
}
finally {
  if (Test-Path -LiteralPath $probeRoot) {
    Remove-Item -LiteralPath $probeRoot -Recurse -Force
  }
}

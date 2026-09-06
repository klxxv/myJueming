//! Ephemeral local bridge used only by the QA MCP/AG-UI subprocess harness.
//!
//! Usage: `qa_fixture_server <nonexistent-temp-project.jm>`. The bearer token
//! comes only from `JUEMING_AGENT_TOKEN`. After emitting its redacted readiness
//! record, the fixture accepts only QA acknowledgement commands on stdin, then
//! stops its listener when it receives `shutdown`.

use std::{
    collections::BTreeSet,
    io::{self, BufRead},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use jueming_agent_transport::{AuthToken, ServerConfig, start_server};
use jueming_application::{AgentCall, LocalAppHost};
use jueming_protocol::{
    CreateProjectRequest, Encoding, ImportProfile, ImportSideRequest, SegmentationMode, TextInput,
};
use serde_json::{Map, Value, json};

fn fixture_request(project_path: &Path) -> CreateProjectRequest {
    let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine);
    CreateProjectRequest {
        project_path: project_path.to_string_lossy().into_owned(),
        name: "QA MCP fixture".into(),
        source: ImportSideRequest {
            language_id: "zh-CN".into(),
            title: "fixture source".into(),
            input: TextInput::Paste {
                label: "fixture source".into(),
                text: "甲\n乙".into(),
            },
            profile: profile.clone(),
        },
        target: ImportSideRequest {
            language_id: "en".into(),
            title: "fixture target".into(),
            input: TextInput::Paste {
                label: "fixture target".into(),
                text: "one\ntwo".into(),
            },
            profile,
        },
    }
}

fn project_path() -> Result<PathBuf, String> {
    let raw = std::env::args()
        .nth(1)
        .ok_or_else(|| "usage: qa_fixture_server <nonexistent-temporary-project.jm>".to_string())?;
    let path = PathBuf::from(raw);
    if path.extension().and_then(|extension| extension.to_str()) != Some("jm") {
        return Err("fixture project path must use the .jm extension".into());
    }
    if path.exists() {
        return Err("fixture refuses to open or overwrite an existing project".into());
    }
    if !path.parent().is_some_and(|parent| parent.is_dir()) {
        return Err("fixture project parent must already exist".into());
    }
    Ok(path)
}

fn params(value: Value) -> Map<String, Value> {
    value
        .as_object()
        .cloned()
        .expect("fixture parameters are objects")
}

fn initialize_native_context(host: &LocalAppHost) -> Result<String, Box<dyn std::error::Error>> {
    let binding = host
        .dispatch_native(AgentCall {
            request_id: "qa-bind-context".into(),
            method: "app.bind_session".into(),
            params: Map::new(),
            binding_id: None,
        })?
        .data["binding_id"]
        .as_str()
        .ok_or("fixture binding response was incomplete")?
        .to_owned();
    let segment_id = host.current_snapshot()?.segments[0].segment_id.to_string();
    let published = host.dispatch_native(AgentCall {
        request_id: "qa-publish-context".into(),
        method: "ui.publish_context".into(),
        params: params(json!({
            "tab": "review",
            "mode": null,
            "project_id": null,
            "revision_id": null,
            "binding_id": null,
            "window_focused": true,
            "focused_control": "qa-fixture",
            "selected_text": "",
            "segment_ids": [segment_id],
            "alignment_ids": [],
            "text_range": null,
            "captured_at": "2026-09-06T00:00:00Z"
        })),
        binding_id: Some(binding.clone()),
    })?;
    if published.data["segment_ids"]
        .as_array()
        .is_none_or(Vec::is_empty)
    {
        return Err("fixture top-level context payload was not retained".into());
    }
    Ok(binding)
}

fn acknowledge(
    host: &LocalAppHost,
    binding_id: &str,
    operation_id: &str,
    request_id: &str,
) -> Value {
    match host.dispatch_native(AgentCall {
        request_id: format!("qa-native-ack-{operation_id}"),
        method: "ui.ack".into(),
        params: params(json!({
            "operation_id": operation_id,
            "request_id": request_id,
            "status": "ui_applied"
        })),
        binding_id: Some(binding_id.into()),
    }) {
        Ok(reply) => json!({
            "qa": "native_ack",
            "ok": true,
            "status": reply.data["operation"]["status"].as_str().unwrap_or_default(),
        }),
        Err(error) => json!({ "qa": "native_ack", "ok": false, "code": error.code }),
    }
}

fn bind_native(
    host: &LocalAppHost,
    request_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    host.dispatch_native(AgentCall {
        request_id: request_id.into(),
        method: "app.bind_session".into(),
        params: Map::new(),
        binding_id: None,
    })?
    .data["binding_id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "fixture native binding response was incomplete".into())
}

fn command_result(
    host: &LocalAppHost,
    native_binding: &str,
    wrong_project_path: &Path,
    line: &str,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    if line == "shutdown" {
        return Ok(None);
    }
    let mut parts = line.split_whitespace();
    let command = parts.next().unwrap_or_default();
    let operation_id = parts.next().unwrap_or_default();
    let request_id = parts.next().unwrap_or_default();
    if operation_id.is_empty() || request_id.is_empty() || parts.next().is_some() {
        return Ok(Some(
            json!({ "qa": "command", "ok": false, "code": "invalid_command" }),
        ));
    }
    match command {
        "ack" => Ok(Some(acknowledge(
            host,
            native_binding,
            operation_id,
            request_id,
        ))),
        "wrong-project-ack" => {
            if wrong_project_path.exists() {
                return Err("fixture wrong-project path already exists".into());
            }
            host.create_project(&fixture_request(wrong_project_path))?;
            let wrong_binding = bind_native(host, "qa-wrong-project-bind")?;
            let mut result = acknowledge(host, &wrong_binding, operation_id, request_id);
            result["qa"] = Value::String("wrong_project_ack".into());
            Ok(Some(result))
        }
        _ => Ok(Some(
            json!({ "qa": "command", "ok": false, "code": "unknown_command" }),
        )),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = project_path().map_err(io::Error::other)?;
    let wrong_project_path = path.with_file_name("wrong-project.jm");
    if wrong_project_path.exists() {
        return Err("fixture wrong-project path must not already exist".into());
    }
    let token = std::env::var("JUEMING_AGENT_TOKEN").map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "JUEMING_AGENT_TOKEN is required",
        )
    })?;
    let host = Arc::new(LocalAppHost::new());
    host.create_project(&fixture_request(&path))?;
    let native_binding = initialize_native_context(&host)?;

    let server = start_server(
        Arc::clone(&host),
        ServerConfig {
            enabled: true,
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            auth_token: AuthToken::new(token)?,
            allowed_origins: BTreeSet::new(),
            max_request_bytes: 64 * 1024,
            request_timeout: Duration::from_secs(3),
        },
    )
    .await?;
    let address = server
        .status()
        .bound_addr
        .ok_or("fixture bridge did not bind")?;
    println!(
        "{}",
        json!({ "ready": true, "endpoint": format!("http://{address}") })
    );

    let (commands, mut received) = tokio::sync::mpsc::unbounded_channel();
    tokio::task::spawn_blocking(move || {
        for line in io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            let shutdown = line == "shutdown";
            if commands.send(line).is_err() {
                break;
            }
            if shutdown {
                break;
            }
        }
    });
    while let Some(line) = received.recv().await {
        match command_result(&host, &native_binding, &wrong_project_path, &line)? {
            Some(result) => println!("{result}"),
            None => break,
        }
    }
    server.stop().await?;
    Ok(())
}

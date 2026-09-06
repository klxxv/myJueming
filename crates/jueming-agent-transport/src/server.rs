use std::{
    collections::BTreeSet,
    convert::Infallible,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{Sse, sse::Event},
    routing::{get, post},
};
#[cfg(test)]
use futures_util::StreamExt;
use futures_util::stream;
use jueming_application::{AgentCall, AgentReply, AppEvent, LocalAppHost};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use subtle::ConstantTimeEq;
use thiserror::Error;
use tokio::{
    net::TcpListener,
    sync::{Mutex as AsyncMutex, oneshot, watch},
    task::JoinHandle,
    time::timeout,
};
use tower_http::limit::RequestBodyLimitLayer;

const CALL_PATH: &str = "/v1/agent/call";
const EVENTS_PATH: &str = "/v1/ag-ui/events";
const DEFAULT_REQUEST_LIMIT_BYTES: usize = 1_048_576;
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const STOP_WAIT: Duration = Duration::from_secs(2);

/// A local bearer token. Its value is intentionally neither `Debug` nor `Display`.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthToken(Arc<[u8]>);

impl AuthToken {
    /// Creates a token from a local-user supplied secret.
    pub fn new(token: impl AsRef<str>) -> Result<Self, TransportError> {
        let token = token.as_ref();
        if token.trim().is_empty() {
            return Err(TransportError::InvalidConfig(
                "the local bridge token must not be empty".into(),
            ));
        }
        Ok(Self(Arc::from(token.as_bytes())))
    }

    /// Exposes the token only to the process that launches the local MCP sidecar.
    /// Callers must keep the returned value out of logs and command-line arguments.
    pub fn expose_to_sidecar(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }
}

/// Settings-owned configuration for the local bridge.
#[derive(Clone)]
pub struct ServerConfig {
    /// Must remain false until a local user enables the bridge in Settings.
    pub enabled: bool,
    /// The only accepted bind address is a loopback address.
    pub bind_addr: SocketAddr,
    /// Required bearer token for every endpoint, including SSE.
    pub auth_token: AuthToken,
    /// Browser origins explicitly permitted to make requests. An absent Origin is valid for a
    /// native sidecar; an unconfigured browser origin is rejected. No CORS headers are emitted.
    pub allowed_origins: BTreeSet<String>,
    /// Maximum JSON request size.
    pub max_request_bytes: usize,
    /// Hard timeout for one host call.
    pub request_timeout: Duration,
}

impl ServerConfig {
    /// Creates a disabled loopback configuration.
    pub fn disabled(auth_token: AuthToken) -> Self {
        Self {
            enabled: false,
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            auth_token,
            allowed_origins: BTreeSet::new(),
            max_request_bytes: DEFAULT_REQUEST_LIMIT_BYTES,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }

    fn validate(&self) -> Result<(), TransportError> {
        if !self.bind_addr.ip().is_loopback() {
            return Err(TransportError::InvalidConfig(
                "the local bridge may bind only a loopback address".into(),
            ));
        }
        if self.max_request_bytes == 0 {
            return Err(TransportError::InvalidConfig(
                "max_request_bytes must be greater than zero".into(),
            ));
        }
        if self.request_timeout.is_zero() {
            return Err(TransportError::InvalidConfig(
                "request_timeout must be greater than zero".into(),
            ));
        }
        Ok(())
    }
}

/// Observable local-server state for Settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportState {
    Disabled,
    Running,
    Failed,
}

/// A status snapshot; it never contains the authentication token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub state: TransportState,
    pub enabled: bool,
    pub bound_addr: Option<SocketAddr>,
}

impl ServerStatus {
    fn disabled() -> Self {
        Self {
            state: TransportState::Disabled,
            enabled: false,
            bound_addr: None,
        }
    }
}

/// Errors surfaced to the desktop settings façade. Secrets are never embedded in an error.
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("invalid local bridge configuration: {0}")]
    InvalidConfig(String),
    #[error("failed to bind the local bridge: {0}")]
    Bind(#[source] std::io::Error),
    #[error("the local bridge task failed: {0}")]
    Join(#[source] tokio::task::JoinError),
}

struct LiveServer {
    shutdown: oneshot::Sender<()>,
    stream_shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
}

struct SharedState {
    host: Arc<LocalAppHost>,
    config: Mutex<ServerConfig>,
    status: RwLock<ServerStatus>,
    live: Mutex<Option<LiveServer>>,
    lifecycle: AsyncMutex<()>,
    accepting: Arc<AtomicBool>,
}

/// A process-local controller with explicit enable, stop, and status operations.
#[derive(Clone)]
pub struct ServerHandle {
    shared: Arc<SharedState>,
}

/// Creates the controller and starts its listener only when `config.enabled` is true.
///
/// The default construction path is [`ServerConfig::disabled`], so a bridge is not reachable
/// until the desktop's local Settings action invokes [`ServerHandle::set_enabled`].
pub async fn start_server(
    host: Arc<LocalAppHost>,
    config: ServerConfig,
) -> Result<ServerHandle, TransportError> {
    config.validate()?;
    let enabled = config.enabled;
    let handle = ServerHandle {
        shared: Arc::new(SharedState {
            host,
            config: Mutex::new(config),
            status: RwLock::new(ServerStatus::disabled()),
            live: Mutex::new(None),
            lifecycle: AsyncMutex::new(()),
            accepting: Arc::new(AtomicBool::new(false)),
        }),
    };
    if enabled {
        handle.start_listener().await?;
    }
    Ok(handle)
}

impl ServerHandle {
    /// Returns a snapshot suitable for Settings without opening a network connection.
    pub fn status(&self) -> ServerStatus {
        self.shared
            .status
            .read()
            .expect("local bridge status lock poisoned")
            .clone()
    }

    /// Enables or disables the listener. Disabling waits for graceful shutdown.
    pub async fn set_enabled(&self, enabled: bool) -> Result<ServerStatus, TransportError> {
        let _lifecycle = self.shared.lifecycle.lock().await;
        if enabled {
            {
                let mut config = self
                    .shared
                    .config
                    .lock()
                    .expect("local bridge config lock poisoned");
                config.enabled = true;
            }
            self.start_listener().await?;
        } else {
            self.stop_locked().await?;
        }
        Ok(self.status())
    }

    /// Stops the listener and marks the bridge disabled. It is safe to call repeatedly.
    pub async fn stop(&self) -> Result<(), TransportError> {
        let _lifecycle = self.shared.lifecycle.lock().await;
        self.stop_locked().await
    }

    async fn stop_locked(&self) -> Result<(), TransportError> {
        // Revoke authorization before graceful shutdown. Existing keepalive connections can no
        // longer issue work while Settings has disabled the bridge.
        self.shared.accepting.store(false, Ordering::Release);
        {
            let mut config = self
                .shared
                .config
                .lock()
                .expect("local bridge config lock poisoned");
            config.enabled = false;
        }
        let live = self
            .shared
            .live
            .lock()
            .expect("local bridge live lock poisoned")
            .take();
        if let Some(mut live) = live {
            let _ = live.stream_shutdown.send(true);
            let _ = live.shutdown.send(());
            match timeout(STOP_WAIT, &mut live.task).await {
                Ok(result) => result.map_err(TransportError::Join)?,
                Err(_) => {
                    live.task.abort();
                    let _ = live.task.await;
                }
            }
        }
        *self
            .shared
            .status
            .write()
            .expect("local bridge status lock poisoned") = ServerStatus::disabled();
        Ok(())
    }

    async fn start_listener(&self) -> Result<(), TransportError> {
        if self
            .shared
            .live
            .lock()
            .expect("local bridge live lock poisoned")
            .is_some()
        {
            return Ok(());
        }

        let config = self
            .shared
            .config
            .lock()
            .expect("local bridge config lock poisoned")
            .clone();
        config.validate()?;
        let listener = TcpListener::bind(config.bind_addr)
            .await
            .map_err(TransportError::Bind)?;
        let bound_addr = listener.local_addr().map_err(TransportError::Bind)?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (stream_shutdown_tx, stream_shutdown_rx) = watch::channel(false);
        let state = HttpState {
            host: Arc::clone(&self.shared.host),
            guard: AuthGuard {
                auth_token: config.auth_token,
                allowed_origins: config.allowed_origins,
            },
            request_timeout: config.request_timeout,
            accepting: Arc::clone(&self.shared.accepting),
            stream_shutdown: stream_shutdown_rx,
        };
        let router = Router::new()
            .route(CALL_PATH, post(dispatch_call))
            .route(EVENTS_PATH, get(ag_ui_events))
            .layer(RequestBodyLimitLayer::new(config.max_request_bytes))
            .with_state(state);
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await;
        });
        self.shared.accepting.store(true, Ordering::Release);
        *self
            .shared
            .status
            .write()
            .expect("local bridge status lock poisoned") = ServerStatus {
            state: TransportState::Running,
            enabled: true,
            bound_addr: Some(bound_addr),
        };
        *self
            .shared
            .live
            .lock()
            .expect("local bridge live lock poisoned") = Some(LiveServer {
            shutdown: shutdown_tx,
            stream_shutdown: stream_shutdown_tx,
            task,
        });
        Ok(())
    }
}

#[derive(Clone)]
struct HttpState {
    host: Arc<LocalAppHost>,
    guard: AuthGuard,
    request_timeout: Duration,
    accepting: Arc<AtomicBool>,
    stream_shutdown: watch::Receiver<bool>,
}

#[derive(Clone)]
struct AuthGuard {
    auth_token: AuthToken,
    allowed_origins: BTreeSet<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HttpError {
    code: &'static str,
    message: String,
}

type HttpResult<T> = Result<Json<T>, (StatusCode, Json<HttpError>)>;

fn authorize(
    headers: &HeaderMap,
    guard: &AuthGuard,
    accepting: &AtomicBool,
) -> Result<(), (StatusCode, Json<HttpError>)> {
    if !accepting.load(Ordering::Acquire) {
        return Err(http_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "bridge_disabled",
            "the local bridge is disabled",
        ));
    }
    if let Some(origin) = headers.get(header::ORIGIN) {
        let origin = origin.to_str().unwrap_or_default();
        if !guard.allowed_origins.contains(origin) {
            return Err(http_error(
                StatusCode::FORBIDDEN,
                "origin_rejected",
                "this browser origin is not permitted for the local bridge",
            ));
        }
    }
    let candidate = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let valid = candidate.is_some_and(|value| {
        let candidate = value.as_bytes();
        candidate.len() == guard.auth_token.0.len()
            && bool::from(candidate.ct_eq(guard.auth_token.0.as_ref()))
    });
    if valid {
        Ok(())
    } else {
        Err(http_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "a valid local bearer token is required",
        ))
    }
}

fn http_error(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> (StatusCode, Json<HttpError>) {
    (
        status,
        Json(HttpError {
            code,
            message: message.into(),
        }),
    )
}

async fn dispatch_call(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(call): Json<AgentCall>,
) -> HttpResult<AgentReply> {
    authorize(&headers, &state.guard, &state.accepting)?;
    if is_native_only(&call.method) {
        return Err(http_error(
            StatusCode::FORBIDDEN,
            "native_only_method",
            "this application method is available only to the trusted native UI",
        ));
    }
    if requires_binding(&call.method) && call.binding_id.is_none() {
        return Err(http_error(
            StatusCode::BAD_REQUEST,
            "binding_required",
            "external calls must bind explicitly before accessing project state",
        ));
    }
    let host = Arc::clone(&state.host);
    let preflight = requires_binding(&call.method) && call.method != "ui.get_context";
    let reply = timeout(
        state.request_timeout,
        tokio::task::spawn_blocking(move || {
            if preflight {
                host.dispatch(AgentCall {
                    request_id: format!("{}:binding-check", call.request_id),
                    method: "ui.get_context".into(),
                    params: serde_json::Map::new(),
                    binding_id: call.binding_id.clone(),
                })?;
            }
            host.dispatch(call)
        }),
    )
    .await
    .map_err(|_| {
        http_error(
            StatusCode::GATEWAY_TIMEOUT,
            "host_timeout",
            "the host did not reply in time",
        )
    })?
    .map_err(|_| {
        http_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "host_unavailable",
            "the host task stopped",
        )
    })?
    .map_err(app_error)?;
    Ok(Json(reply))
}

fn is_native_only(method: &str) -> bool {
    matches!(
        method,
        "app.get_projection" | "ui.publish_context" | "ui.ack" | "proposal.approve"
    )
}

fn requires_binding(method: &str) -> bool {
    !matches!(method, "app.describe" | "app.bind_session")
}

fn app_error(error: impl Serialize) -> (StatusCode, Json<HttpError>) {
    let value = serde_json::to_value(error).unwrap_or_else(|_| json!({}));
    let code = value
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("host_error");
    let message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("the local host rejected the request");
    let status = if matches!(code, "stale_binding" | "stale_revision") {
        StatusCode::CONFLICT
    } else {
        StatusCode::BAD_REQUEST
    };
    http_error(status, "host_error", format!("{code}: {message}"))
}

async fn ag_ui_events(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<
    Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>,
    (StatusCode, Json<HttpError>),
> {
    authorize(&headers, &state.guard, &state.accepting)?;
    let receiver = state.host.subscribe();
    let stream_shutdown = state.stream_shutdown.clone();
    let events = stream::unfold(
        (receiver, stream_shutdown),
        |(mut receiver, mut stream_shutdown)| async move {
            let received = tokio::select! {
                event = receiver.recv() => Some(event),
                changed = stream_shutdown.changed() => {
                    let _ = changed;
                    None
                }
            };
            match received {
                None => None,
                Some(Ok(event)) => Some((Ok(ag_ui_event(event)), (receiver, stream_shutdown))),
                Some(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {
                    let event = Event::default().event("CUSTOM").data(
                        json!({
                            "type": "CUSTOM",
                            "name": "jueming.resync_required",
                            "value": { "reason": "event_gap" }
                        })
                        .to_string(),
                    );
                    Some((Ok(event), (receiver, stream_shutdown)))
                }
                Some(Err(tokio::sync::broadcast::error::RecvError::Closed)) => None,
            }
        },
    );
    Ok(Sse::new(events))
}

/// Converts host events to AG-UI custom events only. It never manufactures `TEXT_MESSAGE_*`
/// events because this endpoint is an application-event adapter, not a chat provider.
fn ag_ui_event(event: AppEvent) -> Event {
    Event::default().event("CUSTOM").data(
        json!({
            "type": "CUSTOM",
            "name": "jueming.app_event",
            "value": event,
        })
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard() -> AuthGuard {
        AuthGuard {
            auth_token: AuthToken::new("test-token").expect("token"),
            allowed_origins: BTreeSet::from(["http://127.0.0.1:1420".into()]),
        }
    }

    #[test]
    fn rejects_missing_bearer_token() {
        assert!(authorize(&HeaderMap::new(), &guard(), &AtomicBool::new(true)).is_err());
    }

    #[test]
    fn accepts_exact_token_and_configured_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer test-token".parse().expect("header"),
        );
        headers.insert(
            header::ORIGIN,
            "http://127.0.0.1:1420".parse().expect("header"),
        );
        assert!(authorize(&headers, &guard(), &AtomicBool::new(true)).is_ok());
    }

    #[test]
    fn rejects_unconfigured_origin_even_with_a_valid_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer test-token".parse().expect("header"),
        );
        headers.insert(
            header::ORIGIN,
            "https://attacker.invalid".parse().expect("header"),
        );
        assert!(authorize(&headers, &guard(), &AtomicBool::new(true)).is_err());
    }

    #[test]
    fn rejects_keepalive_requests_after_acceptance_is_revoked() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer test-token".parse().expect("header"),
        );
        assert!(authorize(&headers, &guard(), &AtomicBool::new(false)).is_err());
    }

    #[test]
    fn keeps_navigation_and_reveal_external_but_blocks_native_only_methods() {
        assert!(!is_native_only("ui.navigate"));
        assert!(!is_native_only("ui.reveal"));
        assert!(is_native_only("ui.publish_context"));
        assert!(is_native_only("proposal.approve"));
        assert!(!requires_binding("app.describe"));
        assert!(!requires_binding("app.bind_session"));
        assert!(requires_binding("ui.navigate"));
    }

    #[tokio::test]
    async fn authenticated_bridge_forwards_safe_calls_and_rejects_native_only_ones() {
        let host = Arc::new(LocalAppHost::new());
        let token = AuthToken::new("integration-token").expect("token");
        let mut config = ServerConfig::disabled(token.clone());
        config.enabled = true;
        let handle = start_server(host, config).await.expect("server starts");
        let address = handle.status().bound_addr.expect("bound loopback address");
        let base = format!("http://{address}{CALL_PATH}");
        let client = reqwest::Client::new();

        let describe = client
            .post(&base)
            .bearer_auth(token.expose_to_sidecar())
            .json(&AgentCall {
                request_id: "describe".into(),
                method: "app.describe".into(),
                params: serde_json::Map::new(),
                binding_id: None,
            })
            .send()
            .await
            .expect("describe request");
        assert_eq!(describe.status(), StatusCode::OK);

        let hostile_origin = client
            .post(&base)
            .bearer_auth(token.expose_to_sidecar())
            .header(header::ORIGIN, "https://attacker.invalid")
            .json(&AgentCall {
                request_id: "hostile-origin".into(),
                method: "app.describe".into(),
                params: serde_json::Map::new(),
                binding_id: None,
            })
            .send()
            .await
            .expect("hostile origin request");
        assert_eq!(hostile_origin.status(), StatusCode::FORBIDDEN);

        let native_only = client
            .post(&base)
            .bearer_auth(token.expose_to_sidecar())
            .json(&AgentCall {
                request_id: "projection".into(),
                method: "app.get_projection".into(),
                params: serde_json::Map::new(),
                binding_id: None,
            })
            .send()
            .await
            .expect("projection request");
        assert_eq!(native_only.status(), StatusCode::FORBIDDEN);

        let unbound_summary = client
            .post(&base)
            .bearer_auth(token.expose_to_sidecar())
            .json(&AgentCall {
                request_id: "summary".into(),
                method: "project.get_summary".into(),
                params: serde_json::Map::new(),
                binding_id: None,
            })
            .send()
            .await
            .expect("summary request");
        assert_eq!(unbound_summary.status(), StatusCode::BAD_REQUEST);

        let unauthenticated = client
            .post(&base)
            .json(&AgentCall {
                request_id: "unauthenticated".into(),
                method: "app.describe".into(),
                params: serde_json::Map::new(),
                binding_id: None,
            })
            .send()
            .await
            .expect("unauthenticated request");
        assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

        handle.stop().await.expect("server stops");
        assert_eq!(handle.status().state, TransportState::Disabled);
    }

    #[tokio::test]
    async fn bridge_rejects_a_body_over_its_configured_limit() {
        let token = AuthToken::new("limit-token").expect("token");
        let mut config = ServerConfig::disabled(token.clone());
        config.enabled = true;
        config.max_request_bytes = 256;
        let handle = start_server(Arc::new(LocalAppHost::new()), config)
            .await
            .expect("server starts");
        let address = handle.status().bound_addr.expect("bound loopback address");
        let response = reqwest::Client::new()
            .post(format!("http://{address}{CALL_PATH}"))
            .bearer_auth(token.expose_to_sidecar())
            .json(&AgentCall {
                request_id: "oversized-body".into(),
                method: "app.describe".into(),
                params: serde_json::Map::from_iter([(
                    "padding".into(),
                    Value::String("x".repeat(1024)),
                )]),
                binding_id: None,
            })
            .send()
            .await
            .expect("oversized request response");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        handle.stop().await.expect("server stops");
    }

    #[tokio::test]
    async fn disabling_closes_active_sse_and_serializes_concurrent_toggles() {
        let token = AuthToken::new("stream-token").expect("token");
        let handle = start_server(
            Arc::new(LocalAppHost::new()),
            ServerConfig::disabled(token.clone()),
        )
        .await
        .expect("disabled controller starts");
        let (first, second) = tokio::join!(handle.set_enabled(true), handle.set_enabled(true));
        let first = first.expect("first enable");
        let second = second.expect("second enable");
        assert_eq!(first.bound_addr, second.bound_addr);
        let address = first.bound_addr.expect("loopback listener");
        let client = reqwest::Client::new();
        let events = client
            .get(format!("http://{address}{EVENTS_PATH}"))
            .bearer_auth(token.expose_to_sidecar())
            .send()
            .await
            .expect("SSE opens");
        assert_eq!(events.status(), StatusCode::OK);
        let mut body = events.bytes_stream();

        let (first, second) = tokio::join!(handle.set_enabled(false), handle.set_enabled(false));
        assert_eq!(
            first.expect("first disable").state,
            TransportState::Disabled
        );
        assert_eq!(
            second.expect("second disable").state,
            TransportState::Disabled
        );
        assert!(timeout(Duration::from_secs(1), body.next()).await.is_ok());

        let address = handle
            .set_enabled(true)
            .await
            .expect("reenable")
            .bound_addr
            .expect("new listener");
        let response = client
            .post(format!("http://{address}{CALL_PATH}"))
            .bearer_auth(token.expose_to_sidecar())
            .json(&AgentCall {
                request_id: "after-reenable".into(),
                method: "app.describe".into(),
                params: serde_json::Map::new(),
                binding_id: None,
            })
            .send()
            .await
            .expect("describe after reenable");
        assert_eq!(response.status(), StatusCode::OK);
        handle.stop().await.expect("final stop");
    }

    #[test]
    fn rejects_non_loopback_configuration() {
        let config = ServerConfig {
            enabled: false,
            bind_addr: "0.0.0.0:1234".parse().expect("socket address"),
            auth_token: AuthToken::new("test-token").expect("token"),
            allowed_origins: BTreeSet::new(),
            max_request_bytes: 1,
            request_timeout: Duration::from_secs(1),
        };
        assert!(config.validate().is_err());
    }
}

use std::{
    fs,
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{
    extract::{ConnectInfo, Path, Request, State},
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    firewall,
    model::{Config, Forward, ForwardInput},
    tailscale,
};

pub struct AppState {
    pub config_path: PathBuf,
    pub config: Mutex<Config>,
    pub tailnet_ip: Ipv4Addr,
}

#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

fn bad_request(message: impl ToString) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message.to_string())
}

fn internal(error: impl ToString) -> ApiError {
    tracing::error!(error = %error.to_string(), "API operation failed");
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(status))
        .route("/nodes", get(nodes))
        .route("/forwards", get(forwards).post(create_forward))
        .route(
            "/forwards/{id}",
            axum::routing::put(update_forward).delete(delete_forward),
        )
        .route("/firewall", get(live_firewall))
        .layer(axum::middleware::from_fn_with_state(state, authorize))
}

pub async fn authorize(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|address| address.0.ip());
    let Some(IpAddr::V4(ip)) = peer else {
        return ApiError(
            StatusCode::FORBIDDEN,
            "Tailscale IPv4 access required".into(),
        )
        .into_response();
    };
    if !crate::model::is_tailnet_ip(ip) {
        return ApiError(
            StatusCode::FORBIDDEN,
            "Tailscale IPv4 access required".into(),
        )
        .into_response();
    }

    let login = match tailscale::whois_login(ip) {
        Ok(login) => login,
        Err(error) => {
            tracing::warn!(peer = %ip, error = %error, "Tailscale identity lookup failed");
            return ApiError(
                StatusCode::FORBIDDEN,
                "Tailscale identity unavailable".into(),
            )
            .into_response();
        }
    };
    let config = state.config.lock().await;
    let permitted = config
        .allowed_users
        .iter()
        .any(|user| user.eq_ignore_ascii_case(&login));
    drop(config);
    if !permitted {
        return ApiError(
            StatusCode::FORBIDDEN,
            "Tailscale user is not allowed".into(),
        )
        .into_response();
    }

    if !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) {
        if let Some(origin) = request.headers().get(header::ORIGIN) {
            let host = request.headers().get(header::HOST);
            let expected = host
                .and_then(|host| host.to_str().ok())
                .map(|host| format!("http://{host}"));
            if origin.to_str().ok() != expected.as_deref() {
                return ApiError(
                    StatusCode::FORBIDDEN,
                    "cross-origin mutation rejected".into(),
                )
                .into_response();
            }
        }
    }
    next.run(request).await
}

#[derive(Serialize)]
struct Status {
    version: &'static str,
    tailnet_ip: Ipv4Addr,
    configured_forwards: usize,
    enabled_forwards: usize,
    live_managed_rules: usize,
    in_sync: bool,
}

async fn status(State(state): State<Arc<AppState>>) -> Result<Json<Status>, ApiError> {
    let config = state.config.lock().await;
    let live = firewall::live_rules().map_err(internal)?;
    let managed: Vec<_> = live.iter().filter(|rule| rule.managed).collect();
    let enabled: Vec<_> = config
        .forwards
        .iter()
        .filter(|forward| forward.enabled)
        .collect();
    let in_sync = managed.len() == enabled.len()
        && enabled.iter().all(|forward| {
            managed.iter().any(|rule| {
                rule.protocol == forward.protocol.as_str()
                    && rule.public_port == forward.public_port
                    && rule.destination == format!("{}:{}", forward.target_ip, forward.target_port)
            })
        });
    Ok(Json(Status {
        version: env!("CARGO_PKG_VERSION"),
        tailnet_ip: state.tailnet_ip,
        configured_forwards: config.forwards.len(),
        enabled_forwards: enabled.len(),
        live_managed_rules: managed.len(),
        in_sync,
    }))
}

async fn nodes() -> Result<Json<Vec<tailscale::Node>>, ApiError> {
    tailscale::nodes().map(Json).map_err(internal)
}

async fn forwards(State(state): State<Arc<AppState>>) -> Json<Vec<Forward>> {
    Json(state.config.lock().await.forwards.clone())
}

async fn live_firewall() -> Result<Json<Vec<firewall::LiveRule>>, ApiError> {
    firewall::live_rules().map(Json).map_err(internal)
}

fn resolve_forward(input: ForwardInput, id: Uuid) -> Result<Forward, ApiError> {
    let node = tailscale::nodes()
        .map_err(internal)?
        .into_iter()
        .find(|node| node.id == input.node_id)
        .ok_or_else(|| bad_request("destination node is not present in the tailnet"))?;
    Ok(Forward {
        id,
        label: input.label.trim().to_owned(),
        protocol: input.protocol,
        public_port: input.public_port,
        node_id: node.id,
        node_name: node.name,
        target_ip: node.ipv4,
        target_port: input.target_port,
        enabled: input.enabled,
    })
}

fn persist(path: &PathBuf, config: &Config) -> Result<()> {
    let directory = path.parent().context("config path has no parent")?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    temporary.write_all(&serde_json::to_vec_pretty(config)?)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    fs::File::open(directory)?.sync_all()?;
    Ok(())
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn apply_change(state: &AppState, current: &mut Config, next: Config) -> Result<(), ApiError> {
    next.validate().map_err(bad_request)?;
    if let Err(error) = firewall::apply(&next) {
        // A failed apply retains the temporary fail-closed FORWARD guard.
        if let Err(rollback_error) = firewall::apply(current) {
            tracing::error!(error = %rollback_error, "firewall rollback failed; guard retained");
        }
        return Err(internal(error));
    }
    if let Err(error) = persist(&state.config_path, &next) {
        if let Err(rollback_error) = firewall::apply(current) {
            tracing::error!(error = %rollback_error, "firewall rollback failed after save failure");
        }
        return Err(internal(error));
    }
    *current = next;
    Ok(())
}

async fn create_forward(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ForwardInput>,
) -> Result<(StatusCode, Json<Forward>), ApiError> {
    let forward = resolve_forward(input, Uuid::new_v4())?;
    let mut current = state.config.lock().await;
    let mut next = current.clone();
    next.forwards.push(forward.clone());
    apply_change(&state, &mut current, next)?;
    Ok((StatusCode::CREATED, Json(forward)))
}

async fn update_forward(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(input): Json<ForwardInput>,
) -> Result<Json<Forward>, ApiError> {
    let forward = resolve_forward(input, id)?;
    let mut current = state.config.lock().await;
    let mut next = current.clone();
    let target = next
        .forwards
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "forward not found".into()))?;
    *target = forward.clone();
    apply_change(&state, &mut current, next)?;
    Ok(Json(forward))
}

async fn delete_forward(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let mut current = state.config.lock().await;
    let mut next = current.clone();
    next.forwards.retain(|entry| entry.id != id);
    if next.forwards.len() == current.forwards.len() {
        return Err(ApiError(StatusCode::NOT_FOUND, "forward not found".into()));
    }
    apply_change(&state, &mut current, next)?;
    Ok(StatusCode::NO_CONTENT)
}

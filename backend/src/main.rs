mod api;
mod firewall;
mod model;
mod tailscale;

use std::{fs, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use axum::Router;
use tokio::sync::Mutex;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

use crate::{api::AppState, model::Config};

fn argument(flag: &str, default: &str) -> Result<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("tailnat --config PATH --web-dir PATH\n\nBinds HTTP only to this machine's Tailscale IPv4 address.");
        std::process::exit(0);
    }
    if args.len() % 2 == 0
        || args
            .iter()
            .skip(1)
            .step_by(2)
            .any(|arg| arg != "--config" && arg != "--web-dir")
    {
        bail!("usage: tailnat [--config PATH] [--web-dir PATH]");
    }
    Ok(args
        .windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| PathBuf::from(&pair[1]))
        .unwrap_or_else(|| PathBuf::from(default)))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tailnat=info,tower_http=info".into()),
        )
        .init();

    let config_path = argument("--config", "/etc/tailnat/config.json")?;
    let web_dir = argument("--web-dir", "/opt/tailnat/web")?;
    let config: Config = serde_json::from_slice(
        &fs::read(&config_path).with_context(|| format!("reading {}", config_path.display()))?,
    )?;
    config.validate()?;
    if !web_dir.join("index.html").is_file() {
        bail!("web directory has no index.html: {}", web_dir.display());
    }

    let tailnet_ip = tailscale::own_ipv4()?;
    firewall::apply(&config).context("applying managed firewall rules")?;
    let listener =
        tokio::net::TcpListener::bind(SocketAddr::from((tailnet_ip, config.listen_port)))
            .await
            .context("binding to Tailscale address")?;

    let state = Arc::new(AppState {
        config_path,
        config: Mutex::new(config),
        tailnet_ip,
    });
    let app = Router::new()
        .nest("/api/v1", api::routes(state.clone()))
        .fallback_service(
            ServeDir::new(&web_dir).fallback(ServeFile::new(web_dir.join("index.html"))),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    tracing::info!(address = %listener.local_addr()?, "TailNAT listening on the tailnet");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("HTTP server stopped")?;
    Ok(())
}

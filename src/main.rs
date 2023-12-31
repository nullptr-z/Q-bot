use std::sync::Arc;

use anyhow::{Ok, Result};
use axum::{
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use q_bot::{
    handlers::{assistant_handler, events_handler, index_page},
    AppState, Args,
};
use tower_http::services::ServeDir;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let state = Arc::new(AppState::default());

    let app = Router::new()
        .route("/", get(index_page))
        .route("/events", get(events_handler))
        .route("/assistant", post(assistant_handler))
        .nest_service("/public", ServeDir::new("./html-ui/public"))
        .nest_service("/assets", ServeDir::new("/tmp/qbot"))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);
    info!("Listening on https://{}", addr);

    let cert = std::fs::read(format!("{}/cert.pem", args.cert_path))?;
    let key = std::fs::read(format!("{}/key.pem", args.cert_path))?;
    let config = RustlsConfig::from_pem(cert, key).await?;
    axum_server::bind_rustls(addr.parse()?, config)
        // Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

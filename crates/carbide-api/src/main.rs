use carbide_api::routes::build_router;
use carbide_api::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,carbide=debug")),
        )
        .json()
        .init();

    tracing::info!("Starting Carbide server...");

    let state = AppState::new();
    let app = build_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let bind_addr = std::env::var("CARBIDE_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Carbide server listening on {}", bind_addr);

    axum::serve(listener, app).await?;
    Ok(())
}

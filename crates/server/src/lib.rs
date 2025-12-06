pub mod api;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

pub async fn start_server(port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .nest("/api", api::routes())
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Control Plane Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub mod api;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use burncloud_database::{create_default_database, Database};
use burncloud_database_router::RouterDatabase;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

pub async fn start_server(port: u16) -> anyhow::Result<()> {
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;

    let state = AppState {
        db: Arc::new(db),
    };

    let app = Router::new()
        .nest("/console", api::routes(state.clone()))
        .merge(burncloud_client::liveview_router(state.db.clone()))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Control Plane Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::routes;

pub fn router() -> Router {
    Router::new()
        .nest("/api", routes::router())
        .layer(CorsLayer::permissive())
}

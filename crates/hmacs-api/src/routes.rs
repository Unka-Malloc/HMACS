pub mod health;
pub mod tasks;
pub mod compute;
pub mod wallet;
pub mod auth;

use axum::Router;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .nest("/api/v1/auth", auth::router(state.clone()))
        .nest("/api/v1/tasks", tasks::router(state.clone()))
        .nest("/api/v1/compute", compute::router(state.clone()))
        .nest("/api/v1/wallet", wallet::router(state.clone()))
        .nest("/ws", crate::ws::ws_router(state))
}

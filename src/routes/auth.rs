use axum::{middleware, routing::post, Router};

use crate::{
    handlers::auth::{introspect, login, logout, logout_all, logout_current_session, logout_my_all_sessions, refresh_token},
    middleware::session_info_extractor::session_info_middleware,
};

pub fn create_routes() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/logout-current-session", post(logout_current_session))
        .route("/logout-my-all-sessions", post(logout_my_all_sessions))
        .route("/logout-all", post(logout_all))
        .route("/refresh-token", post(refresh_token))
        .route("/introspect", post(introspect))
        .layer(middleware::from_fn(session_info_middleware))
}

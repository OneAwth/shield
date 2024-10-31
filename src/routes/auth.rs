use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::{
    handlers::auth::{admin_login, introspect, login, logout, logout_all, logout_current_session, logout_my_all_sessions, refresh_token, register},
    middleware::session_info_extractor::session_info_middleware,
};

pub fn create_routes() -> Router {
    Router::new()
        .route("/admin-login", post(admin_login))
        .route("/login", post(login))
        .route("/logout", get(logout_current_session).post(logout))
        .route("/logout-all", get(logout_my_all_sessions).post(logout_all))
        .route("/register", post(register))
        .route("/refresh-token", post(refresh_token))
        .route("/introspect", post(introspect))
        .layer(middleware::from_fn(session_info_middleware))
}

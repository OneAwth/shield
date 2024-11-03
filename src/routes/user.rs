use axum::{
    routing::{get, patch},
    Router,
};

use crate::handlers::user::{
    add_resources, delete_resource, delete_resource_by_group, delete_user, get_resource_group, get_resource_groups, get_resources, get_user,
    get_users, update_resource, update_resource_group,
};

pub fn create_routes() -> Router {
    Router::new().route("/", get(get_users)).nest(
        "/:user_id",
        Router::new()
            .route("/", get(get_user).delete(delete_user))
            .nest(
                "/resource-group",
                Router::new().route("/", get(get_resource_groups)).nest(
                    "/:resource_group_key",
                    Router::new().route("/", get(get_resource_group).patch(update_resource_group).delete(delete_resource_by_group)),
                ),
            )
            .nest(
                "/resources",
                Router::new()
                    .route("/", get(get_resources).post(add_resources))
                    .route("/:resource_id", patch(update_resource).delete(delete_resource)),
            ),
    )
}

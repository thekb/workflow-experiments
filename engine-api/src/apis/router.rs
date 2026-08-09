use crate::apis::config::state::AppState as ConfigAppState;
use axum::routing::Router;
use std::sync::Arc;

use crate::apis::config::workflow::*;

pub fn get_router(state: Arc<ConfigAppState>) -> Router {
    let app = Router::new().nest("/workflows", get_workflow_route(state));
    return app;
}

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub enum APIError {
    NotFound(String),
    UnAuthorized(String),
    BadRequest(String),
    Internal(String),
    Conflict(String),
}

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            APIError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            APIError::UnAuthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            APIError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            APIError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            APIError::Conflict(msg) => (StatusCode::CONFLICT, msg),
        };
        (status, Json(serde_json::json!({"error": message}))).into_response()
    }
}

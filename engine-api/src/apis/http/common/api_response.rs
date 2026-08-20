use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub enum APIResponse<T: Serialize> {
    Ok(T),
    Created(T),
    Conflict(T),
    NoContent,
}

impl<T: Serialize> IntoResponse for APIResponse<T> {
    fn into_response(self) -> Response {
        match self {
            Self::Ok(data) => (StatusCode::OK, Json(data)).into_response(),
            Self::Created(data) => {
                (StatusCode::CREATED, Json(data)).into_response()
            }
            Self::Conflict(data) => {
                (StatusCode::CONFLICT, Json(data)).into_response()
            }
            Self::NoContent => (StatusCode::NO_CONTENT).into_response(),
        }
    }
}

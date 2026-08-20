use crate::service::config::*;

use tonic::{Code, Status};

impl From<WorkflowError> for Status {
    fn from(value: WorkflowError) -> Self {
        match value {
            WorkflowError::BadRequest(msg) => {
                Status::new(Code::InvalidArgument, msg)
            }
            WorkflowError::Database(_) | WorkflowError::InternalError(_) => {
                Status::new(Code::Internal, "internal server error")
            }
            WorkflowError::IdempotencyConflict => {
                Status::new(Code::AlreadyExists, "idempotency conflict")
            }
            WorkflowError::VersionConflict { current_version } => Status::new(
                Code::FailedPrecondition,
                format!(
                    "workflow version conflict; current version is {current_version}"
                ),
            ),
            WorkflowError::NotFound => {
                Status::new(Code::NotFound, "workflow not found")
            }
        }
    }
}

use sea_orm::DbErr;
use sea_orm::TransactionError;
use thiserror;

#[derive(Debug, thiserror::Error)]
pub enum WorkflowExecutionError {
    #[error("database error: {0}")]
    Database(DbErr),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("idempotency conflict")]
    IdempotencyConflict,
    #[error("not found")]
    NotFound,
}

impl From<TransactionError<WorkflowExecutionError>> for WorkflowExecutionError {
    fn from(error: TransactionError<WorkflowExecutionError>) -> Self {
        match error {
            TransactionError::Connection(error) => Self::Database(error),
            TransactionError::Transaction(error) => error,
        }
    }
}

impl From<DbErr> for WorkflowExecutionError {
    fn from(value: DbErr) -> Self {
        WorkflowExecutionError::Database(value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowTriggerError {
    #[error("database error: {0}")]
    Database(DbErr),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("idempotency conflict")]
    IdempotencyConflict,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("status conflict: {0}")]
    StatusConflict(String),
}

impl From<TransactionError<WorkflowTriggerError>> for WorkflowTriggerError {
    fn from(error: TransactionError<WorkflowTriggerError>) -> Self {
        match error {
            TransactionError::Connection(error) => Self::Database(error),
            TransactionError::Transaction(error) => error,
        }
    }
}

impl From<DbErr> for WorkflowTriggerError {
    fn from(value: DbErr) -> Self {
        WorkflowTriggerError::Database(value)
    }
}

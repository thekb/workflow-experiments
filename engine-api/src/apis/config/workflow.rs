use crate::entities::config::{
    step::WorkflowConfig, workflow::Model as Workflow,
    workflow_version::Model as WorkflowVersion,
};
use crate::service::workflow_service::*;
use axum::routing::{Router, get, post};

use axum::Extension;
use axum::extract::{Json, Path, Query, State};
use axum::http::HeaderMap;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::apis::common::{
    APIError, APIResponse, ItemList, ListMetadata, Pagination,
    X_IDEMPOTENCY_KEY,
};

use super::state::AppState;
use crate::apis::middleware::UserContext;

pub fn get_workflow_route(state: Arc<AppState>) -> Router {
    let router = Router::new()
        .route("/", get(get_workflows))
        .route("/{workflow_id}", get(get_workflow))
        .route("/", post(create_workflow))
        .route("/{workflow_id}/versions", post(create_workflow_version))
        .route("/{workflow_id}/versions", get(get_workflow_versions))
        .route(
            "/{workflow_id}/versions/{version_id}",
            get(get_workflow_version),
        )
        .with_state(state);
    return router;
}

impl From<WorkflowError> for APIError {
    fn from(value: WorkflowError) -> Self {
        match value {
            WorkflowError::BadRequest(val) => APIError::BadRequest(val),
            WorkflowError::NotFound => {
                APIError::NotFound("resource not found".to_owned())
            }
            WorkflowError::Database(val) => APIError::Internal(val.to_string()),
            WorkflowError::IdempotencyConflict => {
                APIError::Conflict("idempotency conflict".to_owned())
            }
            WorkflowError::InternalError(val) => APIError::Internal(val),
            WorkflowError::VersionConflict { current_version } => {
                APIError::Conflict(format!(
                    "version conflict: expected: {current_version}"
                ))
            }
        }
    }
}

async fn get_workflows(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
    Extension(user_context): Extension<UserContext>,
) -> Result<APIResponse<ItemList<Workflow>>, APIError> {
    if !(1..=25).contains(&pagination.page_size) {
        return Err(APIError::BadRequest(
            "page_size must be between 1 and 25".into(),
        ));
    }

    let workflow_cursor: Option<WorkflowCursor> = state.cursor_signer.decode(
        pagination.cursor.as_deref(),
        "list-workflows",
        user_context.tenant_id,
    )?;

    let page: crate::service::ModelPage<Workflow, WorkflowCursor> = state
        .workflows
        .get_workflows(GetWorkflows {
            tenant_id: user_context.tenant_id,
            cursor: workflow_cursor,
            page_size: pagination.page_size,
        })
        .await?;

    let next_token = state.cursor_signer.encode(
        "list-workflows",
        user_context.tenant_id,
        page.next_cursor,
    )?;

    Ok(APIResponse::Ok(ItemList {
        metadata: ListMetadata {
            next_token: next_token,
        },
        items: page.items,
    }))
}

async fn get_workflow(
    State(state): State<Arc<AppState>>,
    Path(workflow_id): Path<Uuid>,
    Extension(user_context): Extension<UserContext>,
) -> Result<APIResponse<Workflow>, APIError> {
    let workflow = state
        .workflows
        .get_workflow(GetWorkflow::ByID {
            id: workflow_id,
            tenant_id: user_context.tenant_id,
        })
        .await?;

    Ok(APIResponse::Ok(workflow))
}

#[derive(Deserialize)]
struct CreateWorkflowRequest {
    pub name: String,
    pub config: WorkflowConfig,
}

async fn create_workflow(
    State(state): State<Arc<AppState>>,
    Extension(user_context): Extension<UserContext>,
    headers: HeaderMap,
    Json(input): Json<CreateWorkflowRequest>,
) -> Result<APIResponse<Workflow>, APIError> {
    let idempotency_key = headers
        .get(X_IDEMPOTENCY_KEY)
        .ok_or_else(|| {
            APIError::BadRequest(format!("missing X-IDEMPOTENCY-KEY"))
        })?
        .to_str()
        .map_err(|_| {
            APIError::BadRequest("invalid X-IDEMPOTENCY-KEY".to_owned())
        })?;

    input
        .config
        .validate()
        .map_err(|err| APIError::BadRequest(err))?;

    let command = CreateWorkflow {
        name: input.name,
        idempotency_key: idempotency_key.to_string(),
        tenant_id: user_context.tenant_id,
        config: input.config,
    };
    let workflow = state.workflows.create_workflow(command).await?;

    Ok(APIResponse::Created(workflow))
}

#[derive(Deserialize)]
struct CreateWorkflowVersionRequest {
    pub config: WorkflowConfig,
    pub expected_current_version: i64,
}

async fn create_workflow_version(
    State(state): State<Arc<AppState>>,
    Extension(user_context): Extension<UserContext>,
    Path(workflow_id): Path<Uuid>,
    Json(input): Json<CreateWorkflowVersionRequest>,
) -> Result<APIResponse<WorkflowVersion>, APIError> {
    input
        .config
        .validate()
        .map_err(|err| APIError::BadRequest(err))?;

    let command = CreateWorkflowVersion {
        workflow_id: workflow_id,
        tenant_id: user_context.tenant_id,
        expected_current_version: input.expected_current_version,
        config: input.config,
    };

    let workflow_version =
        state.workflows.create_workflow_version(command).await?;

    Ok(APIResponse::Created(workflow_version))
}

async fn get_workflow_version(
    State(state): State<Arc<AppState>>,
    Extension(user_context): Extension<UserContext>,
    Path((workflow_id, version_id)): Path<(Uuid, i64)>,
) -> Result<APIResponse<WorkflowVersion>, APIError> {
    let workflow_version = state
        .workflows
        .get_workflow_version(GetWorkflowVersion::ByVersion {
            workflow_id: workflow_id,
            version: version_id,
            tenant_id: user_context.tenant_id,
        })
        .await?;

    Ok(APIResponse::Ok(workflow_version))
}

async fn get_workflow_versions(
    State(state): State<Arc<AppState>>,
    Path(workflow_id): Path<Uuid>,
    Extension(user_context): Extension<UserContext>,
    Query(pagination): Query<Pagination>,
) -> Result<APIResponse<ItemList<WorkflowVersion>>, APIError> {
    if !(1..=25).contains(&pagination.page_size) {
        return Err(APIError::BadRequest(
            "page_size must be between 1 and 25".into(),
        ));
    }

    let workflow_version_cursor: Option<WorkflowVersionCursor> =
        state.cursor_signer.decode(
            pagination.cursor.as_deref(),
            "list-workflow-versions",
            user_context.tenant_id,
        )?;

    let page = state
        .workflows
        .get_workflow_versions(GetWorkflowVersions {
            workflow_id: workflow_id,
            tenant_id: user_context.tenant_id,
            cursor: workflow_version_cursor,
            page_size: pagination.page_size,
        })
        .await?;

    let next_token = state.cursor_signer.encode(
        "list-workflow-versions",
        user_context.tenant_id,
        page.next_cursor,
    )?;

    Ok(APIResponse::Ok(ItemList {
        metadata: ListMetadata {
            next_token: next_token,
        },
        items: page.items,
    }))
}

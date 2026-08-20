use crate::entities::config::step::WorkflowConfig;
use crate::entities::config::{workflow, workflow_version};
use crate::service::common::ModelPage;
use chrono::Utc;
use hex;
use sea_orm::ActiveValue::Set;
use sea_orm::error::DbErr;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, TransactionError, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::boxed::Box;
use thiserror;
use uuid::Uuid;

pub struct CreateWorkflow {
    pub name: String,
    pub idempotency_key: String,
    pub tenant_id: Uuid,
    pub config: WorkflowConfig,
}

pub struct CreateWorkflowVersion {
    pub workflow_id: Uuid,
    pub tenant_id: Uuid,
    pub expected_current_version: i64,
    pub config: WorkflowConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("idempotency conflict")]
    IdempotencyConflict,

    #[error("database error: {0}")]
    Database(DbErr),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    InternalError(String),

    #[error("not found")]
    NotFound,

    #[error("workflow current version is incorrect")]
    VersionConflict { current_version: i64 },
}

impl From<TransactionError<WorkflowError>> for WorkflowError {
    fn from(error: TransactionError<WorkflowError>) -> Self {
        match error {
            TransactionError::Connection(error) => Self::Database(error),
            TransactionError::Transaction(error) => error,
        }
    }
}

impl From<DbErr> for WorkflowError {
    fn from(value: DbErr) -> Self {
        WorkflowError::Database(value)
    }
}

pub fn generate_digest<T: Serialize>(
    value: &T,
) -> Result<String, WorkflowError> {
    let serialized = serde_json::to_vec(value)
        .map_err(|err| WorkflowError::BadRequest(err.to_string()))?;
    Ok(hex::encode(Sha256::digest(serialized)))
}

#[derive(Serialize)]
struct WorkflowDigest<'a> {
    pub tenant_id: &'a Uuid,
    pub name: &'a str,
    pub config: &'a WorkflowConfig,
}

#[derive(Serialize)]
struct WorkflowVersionDigest<'a> {
    pub config: &'a WorkflowConfig,
}

pub struct WorkflowService {
    db: DatabaseConnection,
}

pub enum GetWorkflow {
    ByID {
        id: Uuid,
        tenant_id: Uuid,
    },
    ByIdempotencyKey {
        idempotency_key: String,
        tenant_id: Uuid,
    },
}

async fn get_workflow<T: ConnectionTrait>(
    db: &T,
    command: GetWorkflow,
) -> Result<workflow::Model, WorkflowError> {
    match command {
        GetWorkflow::ByID { id, tenant_id } => {
            return workflow::Entity::find_by_id(id)
                .filter(workflow::Column::TenantId.eq(tenant_id))
                .one(db)
                .await?
                .ok_or(WorkflowError::NotFound);
        }
        GetWorkflow::ByIdempotencyKey {
            idempotency_key,
            tenant_id,
        } => {
            return workflow::Entity::find_by_idx_workflow_idempotency((
                tenant_id,
                idempotency_key,
            ))
            .one(db)
            .await?
            .ok_or(WorkflowError::NotFound);
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct WorkflowCursor {
    pub id: Uuid,
}

pub struct GetWorkflows {
    pub tenant_id: Uuid,
    pub cursor: Option<WorkflowCursor>,
    pub page_size: u64,
}

async fn get_workflows<T: ConnectionTrait>(
    db: &T,
    command: GetWorkflows,
) -> Result<ModelPage<workflow::Model, WorkflowCursor>, WorkflowError> {
    let mut cursor = workflow::Entity::find()
        .filter(workflow::Column::TenantId.eq(command.tenant_id))
        .cursor_by(workflow::Column::Id);
    if let Some(current_cursor) = command.cursor {
        cursor.after(current_cursor.id);
    }

    let mut items = cursor.first(command.page_size + 1).all(db).await?;

    let has_more = items.len() > command.page_size as usize;
    if has_more {
        items.pop();
    }

    let next_cursor = if has_more {
        items.last().map(|item| WorkflowCursor { id: item.id })
    } else {
        None
    };

    Ok(ModelPage { items, next_cursor })
}

pub enum GetWorkflowVersion {
    ByVersion {
        workflow_id: Uuid,
        version: i64,
        tenant_id: Uuid,
    },
}

async fn get_workflow_version<T: ConnectionTrait>(
    db: &T,
    cmd: GetWorkflowVersion,
) -> Result<workflow_version::Model, WorkflowError> {
    match cmd {
        GetWorkflowVersion::ByVersion {
            workflow_id,
            version,
            tenant_id,
        } => {
            return workflow_version::Entity::find_by_workflow_version((
                workflow_id,
                version,
            ))
            .filter(workflow_version::Column::TenantId.eq(tenant_id))
            .one(db)
            .await?
            .ok_or(WorkflowError::NotFound);
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WorkflowVersionCursor {
    pub id: Uuid,
    pub version: i64,
}

pub struct GetWorkflowVersions {
    pub workflow_id: Uuid,
    pub tenant_id: Uuid,
    pub cursor: Option<WorkflowVersionCursor>,
    pub page_size: u64,
}

async fn get_workflow_versions<T: ConnectionTrait>(
    db: &T,
    cmd: GetWorkflowVersions,
) -> Result<
    ModelPage<workflow_version::Model, WorkflowVersionCursor>,
    WorkflowError,
> {
    let mut cursor = workflow_version::Entity::find()
        .filter(workflow_version::Column::TenantId.eq(cmd.tenant_id))
        .cursor_by((
            workflow_version::Column::WorkflowId,
            workflow_version::Column::Version,
        ));

    if let Some(current_cursor) = cmd.cursor {
        cursor.after((current_cursor.id, current_cursor.version));
    }

    let mut items = cursor.first(cmd.page_size + 1).all(db).await?;

    let has_more = items.len() > cmd.page_size as usize;
    if has_more {
        items.pop();
    }

    let next_cursor = if has_more {
        items.last().map(|val| WorkflowVersionCursor {
            id: val.workflow_id,
            version: val.version,
        })
    } else {
        None
    };

    Ok(ModelPage { items, next_cursor })
}

impl WorkflowService {
    pub fn new(db: DatabaseConnection) -> Self {
        WorkflowService { db }
    }

    pub async fn get_workflow(
        &self,
        command: GetWorkflow,
    ) -> Result<workflow::Model, WorkflowError> {
        return get_workflow(&self.db, command).await;
    }

    pub async fn get_workflows(
        &self,
        command: GetWorkflows,
    ) -> Result<ModelPage<workflow::Model, WorkflowCursor>, WorkflowError> {
        return get_workflows(&self.db, command).await;
    }

    pub async fn get_workflow_version(
        &self,
        command: GetWorkflowVersion,
    ) -> Result<workflow_version::Model, WorkflowError> {
        return get_workflow_version(&self.db, command).await;
    }

    pub async fn get_workflow_versions(
        &self,
        command: GetWorkflowVersions,
    ) -> Result<
        ModelPage<workflow_version::Model, WorkflowVersionCursor>,
        WorkflowError,
    > {
        return get_workflow_versions(&self.db, command).await;
    }

    pub async fn create_workflow(
        &self,
        command: CreateWorkflow,
    ) -> Result<workflow::Model, WorkflowError> {
        let workflow_digest: String = generate_digest(&WorkflowDigest {
            tenant_id: &command.tenant_id,
            name: &command.name,
            config: &command.config,
        })?;

        let workflow_version_digest =
            generate_digest(&WorkflowVersionDigest {
                config: &command.config,
            })?;

        let result = self
            .db
            .transaction::<_, _, WorkflowError>(|tx: &sea_orm::DatabaseTransaction| {
                Box::pin(async move {
                    let tenant_id = command.tenant_id;
                    let idempotency_key = command.idempotency_key;

                    let active_workflow: workflow::ActiveModel = workflow::ActiveModel {
                        id: Set(Uuid::now_v7()),
                        name: Set(command.name),
                        idempotency_key: Set(idempotency_key.clone()),
                        tenant_id: Set(tenant_id.clone()),
                        digest: Set(workflow_digest.clone()),
                        current_version: Set(1),
                        created_at: Set(Utc::now()),
                        modified_at: Set(Utc::now()),
                    };

                    let on_conflict_workflow = OnConflict::columns(vec![
                        workflow::Column::TenantId,
                        workflow::Column::IdempotencyKey,
                    ])
                    .do_nothing()
                    .to_owned();

                    workflow::Entity::insert(active_workflow)
                        .on_conflict(on_conflict_workflow)
                        .try_insert()
                        .exec(tx)
                        .await?;

                    let saved_workflow = get_workflow(
                        tx,
                        GetWorkflow::ByIdempotencyKey {
                            idempotency_key: idempotency_key.clone(),
                            tenant_id: tenant_id.clone(),
                        },
                    )
                    .await?;

                    if saved_workflow.digest.ne(&workflow_digest) {
                        return Err(WorkflowError::IdempotencyConflict);
                    }

                    let active_workflow_version: workflow_version::ActiveModel =
                        workflow_version::ActiveModel {
                            id: Set(Uuid::now_v7()),
                            workflow_id: Set(saved_workflow.id),
                            tenant_id: Set(command.tenant_id),
                            version: Set(1),
                            digest: Set(workflow_version_digest.clone()),
                            config: Set(command.config),
                            created_at: Set(Utc::now()),
                        };

                    let on_conflict_workflow_version: OnConflict = OnConflict::columns(vec![
                        workflow_version::Column::WorkflowId,
                        workflow_version::Column::Version,
                    ])
                    .do_nothing()
                    .to_owned();

                    workflow_version::Entity::insert(active_workflow_version)
                        .on_conflict(on_conflict_workflow_version)
                        .try_insert()
                        .exec(tx)
                        .await?;

                    let saved_workflow_version = get_workflow_version(
                        tx,
                        GetWorkflowVersion::ByVersion {
                            workflow_id: saved_workflow.id,
                            version: 1,
                            tenant_id: tenant_id,
                        },
                    )
                    .await?;

                    if saved_workflow_version.version != 1
                        || saved_workflow_version.digest.ne(&workflow_version_digest)
                    {
                        return Err(WorkflowError::IdempotencyConflict);
                    }

                    Ok(saved_workflow)
                })
            })
            .await?;

        return Ok(result);
    }

    pub async fn create_workflow_version(
        &self,
        command: CreateWorkflowVersion,
    ) -> Result<workflow_version::Model, WorkflowError> {
        let workflow_id = command.workflow_id;
        let tenant_id = command.tenant_id;
        let digest = generate_digest(&WorkflowVersionDigest {
            config: &command.config,
        })?;

        let result = self
            .db
            .transaction::<_, _, WorkflowError>(|tx| {
                Box::pin(async move {
                    // get current workflow (checks existence + ownership)
                    let current_workflow = get_workflow(
                        tx,
                        GetWorkflow::ByID {
                            id: workflow_id,
                            tenant_id: tenant_id,
                        },
                    )
                    .await?;

                    // compute next workflow version
                    let saved_current_version =
                        current_workflow.current_version;

                    let next_workflow_version = command
                        .expected_current_version
                        .checked_add(1)
                        .ok_or(WorkflowError::BadRequest(
                            "workflow version overflow".to_owned(),
                        ))?;

                    // update workflow current version
                    let mut current_workflow: workflow::ActiveModel =
                        current_workflow.into();
                    current_workflow.current_version =
                        Set(next_workflow_version);
                    let result = workflow::Entity::update_many()
                        .set(current_workflow)
                        .filter(workflow::Column::Id.eq(workflow_id))
                        .filter(
                            workflow::Column::CurrentVersion
                                .eq(command.expected_current_version),
                        )
                        .exec(tx)
                        .await?;

                    if result.rows_affected != 1 {
                        return Err(WorkflowError::VersionConflict {
                            current_version: saved_current_version,
                        });
                    }

                    let active_workflow_version =
                        workflow_version::ActiveModel {
                            id: Set(Uuid::now_v7()),
                            workflow_id: Set(workflow_id),
                            tenant_id: Set(tenant_id),
                            version: Set(next_workflow_version),
                            config: Set(command.config),
                            digest: Set(digest.clone()),
                            created_at: Set(Utc::now()),
                        };

                    workflow_version::Entity::insert(active_workflow_version)
                        .exec(tx)
                        .await?;

                    let saved_workflow_version: workflow_version::Model =
                        get_workflow_version(
                            tx,
                            GetWorkflowVersion::ByVersion {
                                workflow_id: workflow_id,
                                version: next_workflow_version,
                                tenant_id: tenant_id,
                            },
                        )
                        .await?;

                    Ok(saved_workflow_version)
                })
            })
            .await?;

        return Ok(result);
    }
}

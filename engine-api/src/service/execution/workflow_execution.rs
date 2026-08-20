use crate::entities::config::step::StepConfig;
use crate::entities::execution::*;
use crate::{entities::config::step, service::generate_digest};
use chrono::Utc;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, ModelTrait,
    QueryFilter, Set, TransactionError, TransactionTrait,
};
use std::boxed::Box;

use super::WorkflowExecutionError;

use uuid::Uuid;
pub struct WorkflowExecution {
    db: DatabaseConnection,
}

pub enum GetWorkflowRun {
    ByID {
        workflow_run_id: Uuid,
        tenant_id: Uuid,
        with_steps: bool,
    },
    ByIdempotencyKey {
        idempotency_key: String,
        tenant_id: Uuid,
    },
}

async fn get_workflow_run<T: ConnectionTrait>(
    db: &T,
    q: GetWorkflowRun,
) -> Result<WorkflowRunModelEx, WorkflowExecutionError> {
    match q {
        GetWorkflowRun::ByID {
            workflow_run_id,
            tenant_id,
            with_steps,
        } => {
            let mut loader = WorkflowRunEntity::load();
            loader = loader
                .filter(workflow_run::Column::Id.eq(workflow_run_id))
                .filter(workflow_run::Column::TenantId.eq(tenant_id));
            if with_steps {
                loader = loader.with(WorkflowRunStepEntity);
            }
            let workflow_run = loader
                .one(db)
                .await?
                .ok_or(WorkflowExecutionError::NotFound)?;
            return Ok(workflow_run);
        }
        GetWorkflowRun::ByIdempotencyKey {
            idempotency_key,
            tenant_id,
        } => {
            let workflow_run = WorkflowRunEntity::load()
                .filter_by_idx_workflow_run_idempotency((
                    tenant_id,
                    idempotency_key,
                ))
                .one(db)
                .await?
                .ok_or(WorkflowExecutionError::NotFound)?;
            return Ok(workflow_run);
        }
    }
}

pub enum GetWorkflowRunStep {
    ByID { workflow_run_id: Uuid, name: String },
}

struct CreateWorkflowRunStep {
    pub workflow_run_id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub config: StepConfig,
    pub num_parents: i32,
}

async fn create_workflow_run_step<T: ConnectionTrait>(
    db: &T,
    cmd: CreateWorkflowRunStep,
) -> Result<WorkflowRunStep, WorkflowExecutionError> {
    let active_workflow_run_step = workflow_run_step::ActiveModel {
        id: Set(Uuid::now_v7()),
        workflow_run_id: Set(cmd.workflow_run_id),
        name: Set(cmd.name.to_owned()),
        tenant_id: Set(cmd.tenant_id),
        created_at: Set(Utc::now()),
        updated_at: Set(None),
        config: Set(cmd.config),
        status: Set(WorkflowRunStepStatus::Pending),
        current_attempt: Set(1),
        pending_parents: Set(cmd.num_parents),
    };

    let on_conflict = OnConflict::columns(vec![
        workflow_run_step::Column::WorkflowRunId,
        workflow_run_step::Column::Name,
    ])
    .do_nothing()
    .to_owned();

    WorkflowRunStepEntity::insert(active_workflow_run_step)
        .on_conflict(on_conflict)
        .exec(db)
        .await
        .map_err(|err| WorkflowExecutionError::Database(err))?;

    let saved_workflow_run_step =
        WorkflowRunStepEntity::find_by_idx_workflow_run_step((
            cmd.workflow_run_id,
            cmd.name.to_owned(),
        ))
        .one(db)
        .await
        .map_err(|err| WorkflowExecutionError::Database(err))?
        .ok_or(WorkflowExecutionError::NotFound)?;

    Ok(saved_workflow_run_step)
}
pub struct CreateWorkflowRun {
    pub config: step::WorkflowConfig,
    pub workflow_id: Option<Uuid>,
    pub workflow_version: Option<i64>,
    pub tenant_id: Uuid,
    pub idempotency_key: String,
}

impl WorkflowExecution {
    pub fn new(db: DatabaseConnection) -> Self {
        WorkflowExecution { db }
    }

    pub async fn create_workflow_run(
        &self,
        cmd: CreateWorkflowRun,
    ) -> Result<WorkflowRun, WorkflowExecutionError> {
        cmd.config
            .validate()
            .map_err(|err| WorkflowExecutionError::BadRequest(err))?;
        let digest = generate_digest(&cmd.config)
            .map_err(|err| WorkflowExecutionError::BadRequest(err))?;

        let result = self
            .db
            .transaction::<_, _, WorkflowExecutionError>(|tx| {
                Box::pin(async move {
                    let active_workflow_run = workflow_run::ActiveModel {
                        id: Set(Uuid::now_v7()),
                        workflow_id: Set(cmd.workflow_id),
                        workflow_version_num: Set(cmd.workflow_version),
                        tenant_id: Set(cmd.tenant_id),
                        created_at: Set(Utc::now()),
                        status: Set(WorkflowRunStatus::Pending),
                        idempotency_key: Set(cmd.idempotency_key.to_owned()),
                        digest: Set(digest.to_owned()),
                    };

                    let on_conflict = OnConflict::columns(vec![
                        workflow_run::Column::TenantId,
                        workflow_run::Column::IdempotencyKey,
                    ])
                    .do_nothing()
                    .to_owned();

                    WorkflowRunEntity::insert(active_workflow_run)
                        .on_conflict(on_conflict)
                        .exec(tx)
                        .await?;

                    let saved_workflow_run = get_workflow_run(
                        tx,
                        GetWorkflowRun::ByIdempotencyKey {
                            idempotency_key: cmd.idempotency_key.to_owned(),
                            tenant_id: cmd.tenant_id,
                        },
                    )
                    .await?;

                    if saved_workflow_run.digest.ne(&digest) {
                        return Err(
                            WorkflowExecutionError::IdempotencyConflict,
                        );
                    }

                    Err(WorkflowExecutionError::Internal(
                        "implementing".to_owned(),
                    ))
                })
            })
            .await?;

        Ok(result)
    }
}

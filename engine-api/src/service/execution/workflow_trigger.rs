use super::WorkflowTriggerError;
use crate::entities::execution::workflow_trigger::TriggerPayload;

use crate::entities::execution::workflow_trigger_attempt::{
    TriggerAttemptOutput, TriggerAttemptStatus,
};
use crate::entities::execution::*;
use crate::service::common::generate_digest;
use chrono::Utc;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait,
};
use std::boxed::Box;
use uuid::Uuid;

pub struct CreateTrigger {
    pub payload: TriggerPayload,
    pub tenant_id: Uuid,
    pub idempotency_key: String,
}

pub enum GetTrigger {
    ByID {
        id: Uuid,
        tenant_id: Uuid,
    },
    ByIdempotencyKey {
        tenant_id: Uuid,
        idempotency_key: String,
    },
}

async fn get_trigger<T: ConnectionTrait>(
    db: &T,
    q: GetTrigger,
) -> Result<WorkflowTrigger, WorkflowTriggerError> {
    match q {
        GetTrigger::ByID { id, tenant_id } => {
            WorkflowTriggerEntity::find_by_id(id)
                .filter(workflow_trigger::Column::TenantId.eq(tenant_id))
                .one(db)
                .await?
                .ok_or(WorkflowTriggerError::NotFound)
        }
        GetTrigger::ByIdempotencyKey {
            tenant_id,
            idempotency_key,
        } => WorkflowTriggerEntity::find_by_idx_trigger_idempotency((
            tenant_id,
            idempotency_key.to_owned(),
        ))
        .one(db)
        .await?
        .ok_or(WorkflowTriggerError::NotFound),
    }
}

struct IncrementTriggerAttemptNumber {
    pub trigger_id: Uuid,
    pub current_attempt_number: i32,
    pub tenant_id: Uuid,
}

async fn increment_trigger_attempt_number<T: ConnectionTrait>(
    db: &T,
    cmd: IncrementTriggerAttemptNumber,
) -> Result<(), WorkflowTriggerError> {
    let active_trigger_attempt = workflow_trigger_attempt::ActiveModel {
        attempt_number: Set(cmd.current_attempt_number + 1),
        ..Default::default()
    };

    WorkflowTriggerAttemptEntity::update_many()
        .set(active_trigger_attempt)
        .filter(workflow_trigger_attempt::Column::TenantId.eq(cmd.tenant_id))
        .filter(workflow_trigger_attempt::Column::TriggerId.eq(cmd.trigger_id))
        .filter(
            workflow_trigger_attempt::Column::AttemptNumber
                .eq(cmd.current_attempt_number),
        )
        .exec(db)
        .await?;
    Ok(())
}

pub struct UpdateTriggerStatus {
    pub trigger_id: Uuid,
    pub tenant_id: Uuid,
    pub current_attempt_number: i32,
    pub status: TriggerStatus,
    pub reason: Option<String>,
    pub claimed_by: Uuid,
    pub no_op_in_progress: bool,
}

async fn update_trigger_status<T: ConnectionTrait>(
    db: &T,
    cmd: UpdateTriggerStatus,
) -> Result<(), WorkflowTriggerError> {
    let status = cmd.status;

    let active_trigger = workflow_trigger::ActiveModel {
        status: Set(status.clone()),
        reason: Set(cmd.reason),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    };

    let mut updated = WorkflowTriggerEntity::update_many()
        .set(active_trigger)
        .filter(workflow_trigger::Column::Id.eq(cmd.trigger_id))
        .filter(workflow_trigger::Column::TenantId.eq(cmd.tenant_id))
        .filter(
            workflow_trigger_attempt::Column::AttemptNumber
                .eq(cmd.current_attempt_number),
        );

    updated = match status {
        TriggerStatus::InProgress => updated.filter(
            workflow_trigger::Column::Status.eq(TriggerStatus::Pending),
        ),
        TriggerStatus::Success | TriggerStatus::Failed => updated
            .filter(
                workflow_trigger::Column::Status.eq(TriggerStatus::InProgress),
            )
            .filter(
                workflow_trigger_attempt::Column::ClaimedBy.eq(cmd.claimed_by),
            ),

        TriggerStatus::Pending => !unreachable!(),
    };

    let result = updated.exec(db).await?;
    if result.rows_affected != 1
        && !(status == TriggerStatus::InProgress && cmd.no_op_in_progress)
    {
        return Err(WorkflowTriggerError::NotFound);
    }

    Ok(())
}

pub enum GetTriggerAttempt {
    ByID { trigger_id: Uuid, attempt_num: i32 },
}

async fn get_trigger_attempt<T: ConnectionTrait>(
    db: &T,
    q: GetTriggerAttempt,
) -> Result<WorkflowTriggerAttempt, WorkflowTriggerError> {
    match q {
        GetTriggerAttempt::ByID {
            trigger_id,
            attempt_num,
        } => {
            WorkflowTriggerAttemptEntity::find_by_id((trigger_id, attempt_num))
                .one(db)
                .await?
                .ok_or(WorkflowTriggerError::NotFound)
        }
    }
}

pub struct CreateTriggerAttempt {
    pub trigger_id: Uuid,
    pub current_attempt_num: i32,
    pub tenant_id: Uuid,
}

async fn create_trigger_attempt<T: ConnectionTrait>(
    db: &T,
    cmd: CreateTriggerAttempt,
) -> Result<WorkflowTriggerAttempt, WorkflowTriggerError> {
    let next_attempt_num = cmd.current_attempt_num + 1;
    let active_trigger_attempt = workflow_trigger_attempt::ActiveModel {
        trigger_id: Set(cmd.trigger_id),
        attempt_number: Set(next_attempt_num),
        tenant_id: Set(cmd.tenant_id),
        created_at: Set(Utc::now()),
        ..Default::default()
    };

    WorkflowTriggerAttemptEntity::insert(active_trigger_attempt)
        .exec(db)
        .await?;

    return get_trigger_attempt(
        db,
        GetTriggerAttempt::ByID {
            trigger_id: cmd.trigger_id,
            attempt_num: next_attempt_num,
        },
    )
    .await;
}

pub struct UpdateTriggerAttemptStatus {
    pub tenant_id: Uuid,
    pub trigger_id: Uuid,
    pub attempt_number: i32,
    pub status: TriggerAttemptStatus,
    pub claimed_by: Uuid,
    pub output: Option<TriggerAttemptOutput>,
    pub reason: Option<String>,
}

async fn update_trigger_attempt_status<T: ConnectionTrait>(
    db: &T,
    cmd: UpdateTriggerAttemptStatus,
) -> Result<WorkflowTriggerAttempt, WorkflowTriggerError> {
    let mut active_trigger_attempt = workflow_trigger_attempt::ActiveModel {
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    };

    match cmd.status {
        TriggerAttemptStatus::Pending => {
            return Err(WorkflowTriggerError::BadRequest(format!(
                "cannot update trigger attempt status to PENDING"
            )));
        }
        TriggerAttemptStatus::InProgress => {
            active_trigger_attempt.status = Set(cmd.status);
            active_trigger_attempt.claimed_by = Set(Some(cmd.claimed_by));
        }
        TriggerAttemptStatus::Success => {
            active_trigger_attempt.status = Set(cmd.status);
            active_trigger_attempt.output = Set(cmd.output);
        }
        TriggerAttemptStatus::Failed => {
            active_trigger_attempt.status = Set(cmd.status);
            active_trigger_attempt.reason = Set(cmd.reason);
        }
    }
    let mut updated = WorkflowTriggerAttemptEntity::update_many()
        .set(active_trigger_attempt)
        .filter(workflow_trigger_attempt::Column::TenantId.eq(cmd.tenant_id))
        .filter(workflow_trigger_attempt::Column::TriggerId.eq(cmd.trigger_id))
        .filter(
            workflow_trigger_attempt::Column::AttemptNumber
                .eq(cmd.attempt_number),
        );

    updated = match cmd.status {
        TriggerAttemptStatus::InProgress => updated.filter(
            workflow_trigger_attempt::Column::Status
                .eq(TriggerAttemptStatus::Pending),
        ),
        TriggerAttemptStatus::Success | TriggerAttemptStatus::Failed => updated
            .filter(
                workflow_trigger_attempt::Column::Status
                    .eq(TriggerAttemptStatus::InProgress),
            )
            .filter(
                workflow_trigger_attempt::Column::ClaimedBy.eq(cmd.claimed_by),
            ),
        TriggerAttemptStatus::Pending => unreachable!(),
    };

    let result = updated.exec(db).await?;
    if result.rows_affected.ne(&1) {
        return Err(WorkflowTriggerError::StatusConflict(format!(
            "invalid update"
        )));
    }

    return get_trigger_attempt(
        db,
        GetTriggerAttempt::ByID {
            trigger_id: cmd.trigger_id,
            attempt_num: cmd.attempt_number,
        },
    )
    .await;
}

pub struct ClaimTriggerAttempt {
    pub tenant_id: Uuid,
    pub trigger_id: Uuid,
    pub attempt_number: i32,
    pub claimed_by: Uuid,
}

async fn claim_trigger_attempt<T: ConnectionTrait>(
    db: &T,
    cmd: ClaimTriggerAttempt,
) -> Result<WorkflowTriggerAttempt, WorkflowTriggerError> {
    let active = workflow_trigger_attempt::ActiveModel {
        status: Set(TriggerAttemptStatus::InProgress),
        claimed_by: Set(Some(cmd.claimed_by)),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    };

    let result = WorkflowTriggerAttemptEntity::update_many()
        .set(active)
        .filter(workflow_trigger_attempt::Column::TenantId.eq(cmd.tenant_id))
        .filter(workflow_trigger_attempt::Column::TriggerId.eq(cmd.trigger_id))
        .filter(
            workflow_trigger_attempt::Column::AttemptNumber
                .eq(cmd.attempt_number),
        )
        .filter(
            workflow_trigger_attempt::Column::Status
                .eq(TriggerAttemptStatus::Pending),
        )
        .exec(db)
        .await?;

    if result.rows_affected != 1 {
        return Err(WorkflowTriggerError::StatusConflict(
            "attempt is not pending or does not exist".into(),
        ));
    }

    get_trigger_attempt(
        db,
        GetTriggerAttempt::ByID {
            trigger_id: cmd.trigger_id,
            attempt_num: cmd.attempt_number,
        },
    )
    .await
}

pub struct TriggerService {
    db: DatabaseConnection,
    max_attempts: i32,
}

impl TriggerService {
    pub fn new(db: DatabaseConnection, max_attempts: i32) -> Self {
        Self { db, max_attempts }
    }

    pub async fn get_trigger(
        &self,
        cmd: GetTrigger,
    ) -> Result<WorkflowTrigger, WorkflowTriggerError> {
        return get_trigger(&self.db, cmd).await;
    }

    pub async fn create_trigger(
        &self,
        cmd: CreateTrigger,
    ) -> Result<WorkflowTrigger, WorkflowTriggerError> {
        let digest = generate_digest(&cmd.payload)
            .map_err(|err| WorkflowTriggerError::BadRequest(err))?;
        let result = self
            .db
            .transaction::<_, _, WorkflowTriggerError>(|tx| {
                Box::pin(async move {
                    let active_trigger = workflow_trigger::ActiveModel {
                        id: Set(Uuid::now_v7()),
                        tenant_id: Set(cmd.tenant_id),
                        idempotency_key: Set(cmd.idempotency_key.to_owned()),
                        payload: Set(cmd.payload),
                        digest: Set(digest),
                        created_at: Set(Utc::now()),
                        status: Set(TriggerStatus::Pending),
                        reason: Set(None),
                        updated_at: Set(None),
                        current_attempt: Set(1),
                    };

                    let on_conflict = OnConflict::columns(vec![
                        workflow_trigger::Column::TenantId,
                        workflow_trigger::Column::IdempotencyKey,
                    ])
                    .do_nothing()
                    .to_owned();

                    WorkflowTriggerEntity::insert(active_trigger)
                        .on_conflict(on_conflict)
                        .exec(tx)
                        .await?;

                    let saved_trigger = get_trigger(
                        tx,
                        GetTrigger::ByIdempotencyKey {
                            tenant_id: cmd.tenant_id,
                            idempotency_key: cmd.idempotency_key.to_owned(),
                        },
                    )
                    .await?;

                    if saved_trigger.idempotency_key.ne(&cmd.idempotency_key) {
                        return Err(WorkflowTriggerError::IdempotencyConflict);
                    }

                    create_trigger_attempt(
                        tx,
                        CreateTriggerAttempt {
                            trigger_id: saved_trigger.id,
                            current_attempt_num: 0,
                            tenant_id: cmd.tenant_id,
                        },
                    )
                    .await?;

                    Ok(saved_trigger)
                })
            })
            .await?;

        Ok(result)
    }

    pub async fn update_trigger_attempt_status(
        &self,
        cmd: UpdateTriggerAttemptStatus,
    ) -> Result<WorkflowTriggerAttempt, WorkflowTriggerError> {
        match cmd.status {
            // we cannot update to pending/inprogress
            TriggerAttemptStatus::Pending
            | TriggerAttemptStatus::InProgress => {
                return Err(WorkflowTriggerError::StatusConflict(format!(
                    "status: {:?} not allowed",
                    cmd.status
                )));
            }
            _ => {}
        }

        let current_attempt = cmd.attempt_number;
        let max_attempts = self.max_attempts;
        let current_status = cmd.status.clone();
        let trigger_id = cmd.trigger_id;
        let tenant_id = cmd.tenant_id;
        let reason = cmd.reason.clone();
        let claimed_by = cmd.claimed_by;

        let result = self
            .db
            .transaction::<_, _, WorkflowTriggerError>(|tx| {
                Box::pin(async move {
                    update_trigger_attempt_status(tx, cmd).await?;

                    match current_status {
                        TriggerAttemptStatus::Failed => {
                            if current_attempt < max_attempts {
                                // create next attempt
                                create_trigger_attempt(
                                    tx,
                                    CreateTriggerAttempt {
                                        trigger_id: trigger_id,
                                        current_attempt_num: current_attempt
                                            + 1,
                                        tenant_id: tenant_id,
                                    },
                                )
                                .await?;
                                // increment trigger attempt number
                                increment_trigger_attempt_number(
                                    tx,
                                    IncrementTriggerAttemptNumber {
                                        trigger_id: trigger_id,
                                        current_attempt_number: current_attempt,
                                        tenant_id: tenant_id,
                                    },
                                )
                                .await?;
                            } else {
                                // mark trigger failed
                                update_trigger_status(
                                    tx,
                                    UpdateTriggerStatus {
                                        trigger_id: trigger_id,
                                        tenant_id: tenant_id,
                                        status: TriggerStatus::Failed,
                                        reason: reason,
                                        current_attempt_number: current_attempt,
                                        claimed_by: claimed_by,
                                        no_op_in_progress: false,
                                    },
                                )
                                .await?
                            }
                        }
                        TriggerAttemptStatus::Success => {
                            // mark trigger success
                            update_trigger_status(
                                tx,
                                UpdateTriggerStatus {
                                    trigger_id: trigger_id,
                                    tenant_id: tenant_id,
                                    status: TriggerStatus::Success,
                                    reason: reason,
                                    current_attempt_number: current_attempt,
                                    claimed_by: claimed_by,
                                    no_op_in_progress: false,
                                },
                            )
                            .await?
                        }
                        TriggerAttemptStatus::Pending
                        | TriggerAttemptStatus::InProgress => !unreachable!(),
                    }

                    let saved_trigger_attempt = get_trigger_attempt(
                        tx,
                        GetTriggerAttempt::ByID {
                            trigger_id: trigger_id,
                            attempt_num: current_attempt,
                        },
                    )
                    .await?;

                    Ok(saved_trigger_attempt)
                })
            })
            .await?;

        Ok(result)
    }

    pub async fn claim_trigger_attempt(
        &self,
        cmd: ClaimTriggerAttempt,
    ) -> Result<WorkflowTriggerAttempt, WorkflowTriggerError> {
        let claimed_by = cmd.claimed_by;
        let attempt_number = cmd.attempt_number;
        let tenant_id = cmd.tenant_id;
        let trigger_id = cmd.trigger_id;
        self.db
            .transaction::<_, _, WorkflowTriggerError>(|tx| {
                Box::pin(async move {
                    let saved_trigger_attempt =
                        claim_trigger_attempt(tx, cmd).await?;
                    update_trigger_status(
                        tx,
                        UpdateTriggerStatus {
                            trigger_id: trigger_id,
                            tenant_id: tenant_id,
                            current_attempt_number: attempt_number,
                            status: TriggerStatus::InProgress,
                            reason: None,
                            claimed_by: claimed_by,
                            no_op_in_progress: true,
                        },
                    )
                    .await?;

                    Ok(saved_trigger_attempt)
                })
            })
            .await?;

        Err(WorkflowTriggerError::NotFound)
    }
}

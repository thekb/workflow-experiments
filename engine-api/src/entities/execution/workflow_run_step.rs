use crate::entities::config::StepConfig;
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[sea_orm::model]
#[derive(
    Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize,
)]
#[sea_orm(table_name = "workflow_run_steps")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique_key = "idx_workflow_run_step")]
    pub workflow_run_id: Uuid,
    #[sea_orm(unique_key = "idx_workflow_run_step")]
    pub name: String,
    pub tenant_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub config: StepConfig,
    pub status: WorkflowRunStepStatus,
    pub current_attempt: i32,
    pub pending_parents: i32,
    // -- virtual fields
    #[sea_orm(belongs_to, from = "workflow_run_id", to = "id")]
    pub workflow_run: BelongsTo<super::workflow_run::Entity>,
    #[sea_orm(has_many)]
    pub attempts: HasMany<super::workflow_run_step_attempt::Entity>,
    #[sea_orm(
        self_ref,
        via = "workflow_run_step_dependencies",
        from = "Parent",
        to = "Child"
    )]
    pub children: HasMany<Entity>,
    #[sea_orm(self_ref, via = "workflow_run_step_dependencies", reverse)]
    pub parents: HasMany<Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "UPPERCASE"
)]
pub enum WorkflowRunStepStatus {
    Pending,
    Cancelled,
    InProgress,
    Success,
    Failed,
}

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[sea_orm::model]
#[derive(
    Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize,
)]
#[sea_orm(table_name = "workflow_run_step_attempts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique_key = "workflow_run_step")]
    pub workflow_run_step_id: Uuid,
    #[sea_orm(unique_key = "workflow_run_step")]
    pub attempt_number: i32,
    pub tenant_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub status: WorkflowRunStepAttemptStatus,
    pub reason: Option<String>,
    pub claimed_by: Option<String>,
    #[sea_orm(belongs_to, from = "workflow_run_step_id", to = "id")]
    pub workflow_run_step: BelongsTo<super::workflow_run_step::Entity>,
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
pub enum WorkflowRunStepAttemptStatus {
    Pending,
    Cancelled,
    InProgress,
    Success,
    Failed,
}

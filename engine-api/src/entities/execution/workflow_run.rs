use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[sea_orm::model]
#[derive(
    Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize,
)]
#[sea_orm(table_name = "workflow_runs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub workflow_id: Option<Uuid>,
    pub workflow_version_num: Option<i64>,
    #[sea_orm(unique_key = "idx_workflow_run_idempotency")]
    pub tenant_id: Uuid,
    #[sea_orm(unique_key = "idx_workflow_run_idempotency")]
    pub idempotency_key: String,
    pub digest: String,
    pub created_at: DateTime<Utc>,
    pub status: WorkflowRunStatus,
    #[sea_orm(has_many)]
    pub steps: HasMany<super::workflow_run_step::Entity>,
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
pub enum WorkflowRunStatus {
    Pending,
    Cancelled,
    InProgress,
    Success,
    Failed,
}

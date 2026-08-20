use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[sea_orm::model]
#[derive(
    Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize,
)]
#[sea_orm(table_name = "workflow_trigger_attempts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub trigger_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub attempt_number: i32,
    pub tenant_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub status: TriggerAttemptStatus,
    pub claimed_by: Option<Uuid>,
    pub output: Option<TriggerAttemptOutput>,
    pub reason: Option<String>,
}

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromJsonQueryResult,
)]
#[serde(tag = "type", content = "payload")]
pub enum TriggerAttemptOutput {
    Workflow { id: Uuid, version: i64, run: Uuid },
}

#[derive(
    Copy,
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
pub enum TriggerAttemptStatus {
    Pending,
    InProgress,
    Success,
    Failed,
}

impl Default for TriggerAttemptStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl ActiveModelBehavior for ActiveModel {}

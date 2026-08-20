use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromJsonQueryResult,
)]
#[serde(tag = "type", content = "payload")]
pub enum TriggerPayload {
    Workflow { id: Uuid, extra: Option<Json> },
}

#[sea_orm::model]
#[derive(
    Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize,
)]
#[sea_orm(table_name = "workflow_triggers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique_key = "idx_trigger_idempotency")]
    pub tenant_id: Uuid,
    #[sea_orm(unique_key = "idx_trigger_idempotency")]
    pub idempotency_key: String,
    pub payload: TriggerPayload,
    pub digest: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub status: TriggerStatus,
    pub reason: Option<String>,
    pub current_attempt: i64,
}

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
pub enum TriggerStatus {
    Pending,
    InProgress,
    Success,
    Failed,
}
impl ActiveModelBehavior for ActiveModel {}

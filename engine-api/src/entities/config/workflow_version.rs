use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[sea_orm::model]
#[derive(
    Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize,
)]
#[sea_orm(table_name = "workflow_versions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique_key = "workflow_version")]
    pub workflow_id: Uuid,
    #[sea_orm(unique_key = "workflow_version")]
    pub version: i64,
    pub tenant_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub config: super::step::WorkflowConfig,
    #[sea_orm(belongs_to, from = "workflow_id", to = "id")]
    pub workflow: BelongsTo<super::workflow::Entity>,
    pub digest: String,
}

impl ActiveModelBehavior for ActiveModel {}

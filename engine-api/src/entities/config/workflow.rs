use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[sea_orm::model]
#[derive(
    Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize,
)]
#[sea_orm(table_name = "workflows")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub name: String,
    #[sea_orm(unique_key = "idx_workflow_idempotency")]
    pub tenant_id: Uuid,
    #[sea_orm(unique_key = "idx_workflow_idempotency")]
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub current_version: i64,
    pub digest: String,
    #[sea_orm(has_many)]
    pub versions: HasMany<super::workflow_version::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

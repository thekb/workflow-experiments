use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[sea_orm::model]
#[derive(
    Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize,
)]
#[sea_orm(table_name = "workflow_run_step_dependencies")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique_key = "dependency")]
    pub parent_id: Uuid,
    #[sea_orm(unique_key = "dependency")]
    pub child_id: Uuid,
    pub workflow_run_id: Uuid,
    pub tenant_id: Uuid,
    #[sea_orm(
        belongs_to,
        relation_enum = "Parent",
        from = "parent_id",
        to = "id"
    )]
    pub parent: BelongsTo<super::workflow_run_step::Entity>,
    #[sea_orm(
        belongs_to,
        relation_enum = "Child",
        from = "child_id",
        to = "id"
    )]
    pub child: BelongsTo<super::workflow_run_step::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

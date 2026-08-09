mod utilities;
use engine_api::entities::config::{Http, Step, StepConfig, WorkflowConfig};
use engine_api::service::workflow_service::WorkflowService;
use engine_api::service::workflow_service::{
    CreateWorkflow, CreateWorkflowVersion, GetWorkflow, GetWorkflowVersion,
};
use uuid::Uuid;

use utilities::start_postgres;

#[tokio::test]
async fn test_workflow_service_basic() -> Result<(), String> {
    let conn = start_postgres().await?;

    let db = conn.connection();

    db.ping().await.map_err(|err| format!("{err}"))?;

    db.get_schema_registry("engine_api::entities::*")
        .sync(db)
        .await
        .map_err(|err| format!("{err}"))?;

    let ws = WorkflowService::new(db.clone());
    let tenant_id = Uuid::now_v7();
    let created_workflow = ws
        .create_workflow(CreateWorkflow {
            name: "".to_string(),
            idempotency_key: "test".to_string(),
            tenant_id: tenant_id,
            config: WorkflowConfig {
                steps: vec![Step {
                    name: "step_1".to_string(),
                    config: StepConfig::Http(Http {
                        method: "GET".to_string(),
                        url: "http://example.com".to_string(),
                        body: None,
                    }),
                    depends_on: vec![],
                }],
                max_retries: 1,
            },
        })
        .await
        .map_err(|err| format!("{err}"))?;

    ws.get_workflow(GetWorkflow::ByID {
        id: created_workflow.id,
        tenant_id: tenant_id,
    })
    .await
    .map_err(|err| format!("{err}"))?;

    ws.get_workflow_version(GetWorkflowVersion::ByVersion {
        workflow_id: created_workflow.id,
        version: created_workflow.current_version,
        tenant_id: tenant_id,
    })
    .await
    .map_err(|err| format!("{err}"))?;

    let next_version = ws
        .create_workflow_version(CreateWorkflowVersion {
            workflow_id: created_workflow.id,
            tenant_id: tenant_id,
            expected_current_version: created_workflow.current_version,
            config: WorkflowConfig {
                steps: vec![Step {
                    name: "step_2".to_string(),
                    config: StepConfig::Http(Http {
                        method: "GET".to_string(),
                        url: "http://example.com".to_string(),
                        body: None,
                    }),
                    depends_on: vec![],
                }],
                max_retries: 1,
            },
        })
        .await
        .map_err(|err: engine_api::service::WorkflowError| format!("{err}"))?;

    Ok(())
}

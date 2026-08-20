use crate::proto::workflow::v1::*;
use crate::service::config::*;
use tonic::{Code, Request, Response, Status};
use uuid::Uuid;

pub struct WorkflowServer {
    workflows: WorkflowService,
}

impl WorkflowServer {
    pub fn new(workflows: WorkflowService) -> Self {
        WorkflowServer { workflows }
    }
}

#[tonic::async_trait]
impl workflow_config_service_server::WorkflowConfigService for WorkflowServer {
    async fn get_workflow_version(
        &self,
        req: Request<GetWorkflowVersionRequest>,
    ) -> Result<Response<GetWorkflowVersionResponse>, Status> {
        let req = req.into_inner();

        let workflow_id: Uuid = req
            .workflow_id
            .ok_or(Status::new(
                Code::InvalidArgument,
                "workflow_id is required",
            ))?
            .into();

        let tenant_id: Uuid = req
            .tenant_id
            .ok_or(Status::new(Code::InvalidArgument, "tenant_id is required"))?
            .into();

        let version = req.version;

        let result: crate::entities::config::workflow_version::Model = self
            .workflows
            .get_workflow_version(GetWorkflowVersion::ByVersion {
                workflow_id: workflow_id,
                version: version,
                tenant_id: tenant_id,
            })
            .await?;

        Ok(Response::new(GetWorkflowVersionResponse {
            workflow: Some(result.into()),
        }))
    }
}

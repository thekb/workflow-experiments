use crate::proto::workflow::v1::*;
use crate::service::execution::*;
use tonic::{Code, Request, Response, Status};
use uuid::Uuid;

pub struct TriggerServiceServer {
    triggers: TriggerService,
}

impl TriggerServiceServer {
    pub fn new(triggers: TriggerService) -> Self {
        TriggerServiceServer { triggers }
    }
}

#[tonic::async_trait]
impl trigger_service_server::TriggerService for TriggerServiceServer {
    async fn get_next_trigger(
        &self,
        req: Request<GetNextTriggerAttemptRequest>,
    ) -> Result<Response<GetNextTriggerAttemptResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }

    async fn claim_trigger_attempt(
        &self,
        req: Request<ClaimTriggerAttemptRequest>,
    ) -> Result<Response<ClaimTriggerAttemptResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }

    async fn update_trigger_attempt_status(
        &self,
        req: Request<UpdateTriggerAttemptStatusRequest>,
    ) -> Result<Response<UpdateTriggerAttemptStatusResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }
}

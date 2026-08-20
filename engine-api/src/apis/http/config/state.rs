use crate::apis::http::common::CursorSigner;
use crate::service::config::workflow_service::WorkflowService;
use crate::service::execution::TriggerService;

pub struct AppState {
    pub workflows: WorkflowService,
    pub triggers: TriggerService,
    pub cursor_signer: CursorSigner,
}

use crate::{apis::common::CursorSigner, service::WorkflowService};

pub struct AppState {
    pub workflows: WorkflowService,
    pub cursor_signer: CursorSigner,
}

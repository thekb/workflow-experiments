use crate::proto::workflow::v1::{
    ClaimTriggerAttemptRequest, TriggerAttempt, WorkflowPayload,
    trigger_payload::Payload, trigger_service_client::TriggerServiceClient,
    workflow_config_service_client::WorkflowConfigServiceClient,
};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;
use uuid::Uuid;

pub struct TriggerProcessor {
    id: Uuid,
    triggers: TriggerServiceClient<Channel>,
    workflows: WorkflowConfigServiceClient<Channel>,
    num_workers: u32,
    rx: Arc<Mutex<mpsc::Receiver<TriggerAttempt>>>,
    tx: mpsc::Sender<TriggerAttempt>,
}

impl TriggerProcessor {
    pub fn new(
        id: Uuid,
        trigger_client: TriggerServiceClient<Channel>,
        workflow_client: WorkflowConfigServiceClient<Channel>,
        num_workers: u32,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<TriggerAttempt>(num_workers as usize);
        let rx = Arc::new(Mutex::new(rx));

        TriggerProcessor {
            id: id,
            triggers: trigger_client,
            workflows: workflow_client,
            rx: rx,
            tx: tx,
            num_workers: num_workers,
        }
    }

    pub async fn start(token: CancellationToken) -> Result<(), String> {
        Err(format!("not implemented"))
    }

    async fn handle_trigger_attempt(
        &self,
        token: CancellationToken,
        trigger_attempt: TriggerAttempt,
    ) -> Result<(), String> {
        let mut triggers = self.triggers.clone();
        let mut workflows = self.workflows.clone();

        // claim trigger attempt
        let response = triggers
            .claim_trigger_attempt(ClaimTriggerAttemptRequest {
                trigger_id: trigger_attempt.trigger_id,
                attempt_number: trigger_attempt.attempt_number,
                tenant_id: trigger_attempt.tenant_id,
                claimed_by: Some(self.id.into()),
            })
            .await
            .map_err(|err| err.to_string())?;
        // get trigger payload
        let payload = response
            .into_inner()
            .payload
            .ok_or(format!("expected trigger payload"))?;
        // get workflow version version
        // let payload =
        //     payload.payload.ok_or(format!("expected trigger payload"))?;
        // match payload {
        //     Payload::Workflow(wp) => {
        //         let wp.id.
        //     }
        // }
        // create workflow run

        // update status

        Err(format!(""))
    }
}

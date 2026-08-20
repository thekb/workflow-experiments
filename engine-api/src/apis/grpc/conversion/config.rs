use crate::entities::*;

use crate::proto::workflow::v1::*;

impl From<config::Http> for step_config::Config {
    fn from(value: config::Http) -> Self {
        step_config::Config::Http(HttpConfig {
            method: value.method,
            url: value.url,
            body: value.body.unwrap_or("".to_owned()),
        })
    }
}

impl From<config::StepConfig> for StepConfig {
    fn from(value: config::StepConfig) -> Self {
        match value {
            config::StepConfig::Http(http) => StepConfig {
                config: Some(http.into()),
            },
        }
    }
}

impl From<config::Step> for Step {
    fn from(value: config::Step) -> Self {
        Step {
            name: value.name,
            config: Some(value.config.into()),
            depends_on: value.depends_on,
        }
    }
}

impl From<config::WorkflowConfig> for WorkflowConfig {
    fn from(value: config::WorkflowConfig) -> Self {
        WorkflowConfig {
            steps: value.steps.into_iter().map(Into::into).collect(),
            max_retries: value.max_retries,
        }
    }
}

impl From<config::workflow_version::Model> for WorkflowVersion {
    fn from(value: config::workflow_version::Model) -> Self {
        WorkflowVersion {
            id: Some(value.id.into()),
            version: value.version,
            tenant_id: Some(value.tenant_id.into()),
            config: Some(value.config.into()),
            digest: value.digest,
        }
    }
}

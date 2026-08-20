use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

// pub struct Container {
//     pub image: String,
//     pub tag: String,
//     pub command: Vec<String>,
//     pub args: Vec<String>,
// }

#[derive(Serialize, Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct Http {
    #[serde(default = "default_http_method")]
    pub method: String,
    pub url: String,
    pub body: Option<String>,
}

fn default_http_method() -> String {
    String::from("GET")
}

#[derive(
    Clone, Debug, Eq, PartialEq, Serialize, Deserialize, FromJsonQueryResult,
)]
pub enum StepConfig {
    Http(Http),
    // Container,
}
#[derive(
    Clone, Debug, Eq, PartialEq, Serialize, Deserialize, FromJsonQueryResult,
)]
pub struct Step {
    pub name: String,
    pub config: StepConfig,
    #[serde(default)]
    pub depends_on: Vec<String>,
}
#[derive(
    Clone, Debug, Eq, PartialEq, Serialize, Deserialize, FromJsonQueryResult,
)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default = "default_max_retries")]
    pub max_retries: i32,
}

fn default_max_retries() -> i32 {
    3
}

struct WorkflowGraph<'a> {
    adjacency: HashMap<&'a str, Vec<&'a str>>,
    in_degrees: HashMap<&'a str, usize>,
}

impl WorkflowConfig {
    fn build_graph<'a>(&'a self) -> Result<WorkflowGraph<'a>, String> {
        let mut steps = HashMap::new();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut in_degrees = HashMap::new();

        for step in &self.steps {
            if steps.insert(step.name.as_str(), step).is_some() {
                return Err(format!(
                    "step with name {} already exists",
                    step.name
                ));
            }

            in_degrees.insert(step.name.as_str(), 0);
        }

        for step in &self.steps {
            let mut dependencies = HashSet::new();

            for parent in &step.depends_on {
                if !steps.contains_key(parent.as_str()) {
                    return Err(format!("invalid depends_on: {parent}"));
                }

                if !dependencies.insert(parent.as_str()) {
                    return Err(format!(
                        "step {} depends on {} more than once",
                        step.name, parent
                    ));
                }

                adjacency
                    .entry(parent.as_str())
                    .or_default()
                    .push(step.name.as_str());

                *in_degrees
                    .get_mut(step.name.as_str())
                    .expect("step was registered") += 1;
            }
        }

        Ok(WorkflowGraph {
            adjacency,
            in_degrees,
        })
    }

    fn ensure_acyclic<'a>(
        &'a self,
        graph: &WorkflowGraph<'a>,
    ) -> Result<(), String> {
        let mut remaining = graph.in_degrees.clone();
        let mut queue = VecDeque::new();
        let mut visited = 0;

        for (&step, &degree) in &remaining {
            if degree == 0 {
                queue.push_back(step);
            }
        }

        while let Some(step) = queue.pop_front() {
            visited += 1;

            if let Some(children) = graph.adjacency.get(step) {
                for child in children {
                    let degree =
                        remaining.get_mut(child).expect("child was registered");

                    *degree -= 1;

                    if *degree == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }

        if visited != self.steps.len() {
            return Err("workflow contains dependency cycle".into());
        }

        Ok(())
    }

    pub fn in_degrees<'a>(&'a self) -> Result<HashMap<&'a str, usize>, String> {
        let graph = self.build_graph()?;
        self.ensure_acyclic(&graph)?;

        Ok(graph.in_degrees)
    }

    pub fn validate(&self) -> Result<(), String> {
        let graph = self.build_graph()?;
        self.ensure_acyclic(&graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_baseline() -> Result<(), String> {
        let wc = WorkflowConfig {
            steps: vec![Step {
                name: "test".to_owned(),
                config: StepConfig::Http(Http {
                    method: "GET".to_owned(),
                    url: "http://example.com".to_owned(),
                    body: None,
                }),
                depends_on: Vec::new(),
            }],
            max_retries: 1,
        };

        let result = wc.validate();
        if result.is_err() {
            return Err(format!("expected no error"));
        }
        Ok(())
    }

    #[test]
    fn test_check_cycles() -> Result<(), String> {
        let wc: WorkflowConfig = WorkflowConfig {
            steps: vec![
                Step {
                    name: "1".to_owned(),
                    config: StepConfig::Http(Http {
                        method: "GET".to_owned(),
                        url: "http://example.com".to_owned(),
                        body: None,
                    }),
                    depends_on: vec!["2".to_owned()],
                },
                Step {
                    name: "2".to_owned(),
                    config: StepConfig::Http(Http {
                        method: "GET".to_owned(),
                        url: "http://example.com".to_owned(),
                        body: None,
                    }),
                    depends_on: vec!["1".to_owned()],
                },
            ],
            max_retries: 1,
        };

        let result: Result<(), String> = wc.validate();
        if !result.is_err() {
            return Err(format!("expected error"));
        }

        Ok(())
    }

    #[test]
    fn test_duplicate_steps() -> Result<(), String> {
        let wc: WorkflowConfig = WorkflowConfig {
            steps: vec![
                Step {
                    name: "1".to_owned(),
                    config: StepConfig::Http(Http {
                        method: "GET".to_owned(),
                        url: "http://example.com".to_owned(),
                        body: None,
                    }),
                    depends_on: Vec::new(),
                },
                Step {
                    name: "1".to_owned(),
                    config: StepConfig::Http(Http {
                        method: "GET".to_owned(),
                        url: "http://example.com".to_owned(),
                        body: None,
                    }),
                    depends_on: Vec::new(),
                },
            ],
            max_retries: 1,
        };

        let result: Result<(), String> = wc.validate();
        if !result.is_err() {
            return Err(format!("expected error"));
        }

        Ok(())
    }
}

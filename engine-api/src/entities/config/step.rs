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
    pub max_retries: u32,
}

fn default_max_retries() -> u32 {
    3
}

impl WorkflowConfig {
    pub fn validate(&self) -> Result<(), String> {
        let mut steps: HashMap<&str, &Step> = HashMap::new();
        let mut g: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut in_degree: HashMap<&str, i32> = HashMap::new();

        // check duplicates
        for s in &self.steps {
            if steps.contains_key(s.name.as_str()) {
                return Err(format!(
                    "step with name: {} already exists",
                    &s.name
                ));
            }
            steps.insert(&s.name, &s);
            in_degree.entry(&s.name).or_insert(0);
        }

        for s in &self.steps {
            for parent in &s.depends_on {
                // let parent = parent;
                if !steps.contains_key(parent.as_str()) {
                    return Err(format!("invalid depends_on: {}", parent));
                }

                // build the adjacency map
                g.entry(parent.as_str())
                    .or_insert_with(Vec::new)
                    .push(&s.name);

                // add indegree
                let count = in_degree.get_mut(&s.name.as_str()).unwrap();
                *count += 1;
            }
        }

        // ensure there are no cycles
        let mut visited: HashSet<&str> = HashSet::new();
        let mut q: VecDeque<&str> = VecDeque::new();

        for (&key, &value) in &in_degree {
            if value == 0 {
                q.push_back(key);
            }
        }

        while let Some(v) = q.pop_front() {
            visited.insert(v);

            if let Some(neighbors) = g.get(v) {
                for &neighbor in neighbors {
                    let count = in_degree.get_mut(neighbor).unwrap();
                    *count -= 1;
                    if *count == 0 {
                        q.push_back(neighbor);
                    }
                }
            }
        }

        if visited.len() != self.steps.len() {
            return Err("workflow contains dependency cycle".into());
        }

        Ok(())
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

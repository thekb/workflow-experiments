use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub cursor: Option<String>,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page_size() -> u64 {
    10
}

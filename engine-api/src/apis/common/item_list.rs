use serde::Serialize;

#[derive(Serialize)]
pub struct ListMetadata {
    pub next_token: Option<String>,
}

#[derive(Serialize)]
pub struct ItemList<T: Serialize> {
    pub metadata: ListMetadata,
    pub items: Vec<T>,
}

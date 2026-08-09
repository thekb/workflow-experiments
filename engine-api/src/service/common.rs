use serde::Serialize;

#[derive(Serialize)]
pub struct ModelPage<T, C> {
    pub items: Vec<T>,
    pub next_cursor: Option<C>,
}

impl<T, C> ModelPage<T, C> {
    pub fn new(items: Vec<T>, cursor_from_item: impl FnOnce(&T) -> C) -> Self {
        let next_cursor = items.last().map(cursor_from_item);
        Self { items, next_cursor }
    }
}

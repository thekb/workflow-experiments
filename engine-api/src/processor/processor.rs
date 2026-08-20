use async_trait::async_trait;

#[async_trait]
pub trait Processor<T> {
    async fn process(item: T) -> Result<(), String>;
}

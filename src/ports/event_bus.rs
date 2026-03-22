use async_trait::async_trait;

use crate::errors::AppError;

#[async_trait]
pub trait EventStream: Send {
    async fn next(&mut self) -> Option<serde_json::Value>;
}

#[async_trait]
pub trait EventBus: Send + Sync + 'static {
    async fn publish(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), AppError>;
    async fn subscribe(
        &self,
        event_type: &str,
    ) -> Result<Box<dyn EventStream>, AppError>;
}

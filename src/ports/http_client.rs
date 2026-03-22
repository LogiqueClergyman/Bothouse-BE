use async_trait::async_trait;

use crate::errors::AppError;

#[async_trait]
pub trait HttpClient: Send + Sync + 'static {
    async fn post_json(&self, url: &str, body: &serde_json::Value) -> Result<u16, AppError>;
}

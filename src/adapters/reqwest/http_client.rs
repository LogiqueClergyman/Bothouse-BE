use async_trait::async_trait;
use std::time::Duration;

use crate::errors::AppError;
use crate::ports::http_client::HttpClient;

pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Failed to build reqwest client");
        Self { client }
    }
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn post_json(&self, url: &str, body: &serde_json::Value) -> Result<u16, AppError> {
        let resp = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        Ok(resp.status().as_u16())
    }
}

use async_trait::async_trait;
use futures::StreamExt;
use redis::aio::PubSub;

use crate::errors::AppError;
use crate::ports::event_bus::{EventBus, EventStream};

pub struct RedisEventBus {
    client: redis::Client,
    conn: redis::aio::ConnectionManager,
}

impl RedisEventBus {
    pub fn new(client: redis::Client, conn: redis::aio::ConnectionManager) -> Self {
        Self { client, conn }
    }
}

pub struct RedisEventStream {
    pubsub: PubSub,
}

#[async_trait]
impl EventStream for RedisEventStream {
    async fn next(&mut self) -> Option<serde_json::Value> {
        loop {
            let msg = self.pubsub.on_message().next().await?;
            if let Ok(payload) = msg.get_payload::<String>() {
                if let Ok(val) = serde_json::from_str(&payload) {
                    return Some(val);
                }
            }
        }
    }
}

#[async_trait]
impl EventBus for RedisEventBus {
    async fn publish(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        let payload_str = serde_json::to_string(payload).map_err(|e| AppError::Internal(e.into()))?;
        redis::cmd("PUBLISH")
            .arg(event_type)
            .arg(payload_str)
            .query_async::<redis::aio::ConnectionManager, i64>(&mut conn)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        Ok(())
    }

    async fn subscribe(&self, event_type: &str) -> Result<Box<dyn EventStream>, AppError> {
        let mut pubsub = self
            .client
            .get_async_pubsub()
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        pubsub
            .subscribe(event_type)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        Ok(Box::new(RedisEventStream { pubsub }))
    }
}

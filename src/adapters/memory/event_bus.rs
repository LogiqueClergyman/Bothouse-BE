use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;

use crate::errors::AppError;
use crate::ports::event_bus::{EventBus, EventStream};

pub struct MemoryEventBus {
    senders: Mutex<HashMap<String, broadcast::Sender<serde_json::Value>>>,
}

impl MemoryEventBus {
    pub fn new() -> Self {
        Self {
            senders: Mutex::new(HashMap::new()),
        }
    }

    fn get_or_create_sender(&self, event_type: &str) -> broadcast::Sender<serde_json::Value> {
        let mut senders = self.senders.lock().unwrap();
        senders
            .entry(event_type.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(256);
                tx
            })
            .clone()
    }
}

impl Default for MemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemoryEventStream {
    rx: broadcast::Receiver<serde_json::Value>,
}

#[async_trait]
impl EventStream for MemoryEventStream {
    async fn next(&mut self) -> Option<serde_json::Value> {
        self.rx.recv().await.ok()
    }
}

#[async_trait]
impl EventBus for MemoryEventBus {
    async fn publish(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), AppError> {
        let sender = self.get_or_create_sender(event_type);
        let _ = sender.send(payload.clone());
        Ok(())
    }

    async fn subscribe(&self, event_type: &str) -> Result<Box<dyn EventStream>, AppError> {
        let sender = self.get_or_create_sender(event_type);
        let rx = sender.subscribe();
        Ok(Box::new(MemoryEventStream { rx }))
    }
}

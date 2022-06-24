use async_trait::async_trait;
use log::*;
use mqtt_router::{RouteHandler, RouterError};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::driver::Blinds;

pub struct BlindsHandler {
    blinds: Arc<Mutex<Box<dyn Blinds>>>,
}

impl BlindsHandler {
    pub fn new(blinds: Arc<Mutex<Box<dyn Blinds>>>) -> Box<Self> {
        Box::new(Self { blinds })
    }
}

#[async_trait]
impl RouteHandler for BlindsHandler {
    async fn call(&mut self, topic: &str, _content: &[u8]) -> std::result::Result<(), RouterError> {
        info!("got mqtt message on {topic}");
        if topic.ends_with("open") {
            info!("Opening blinds");
            self.blinds
                .lock()
                .await
                .open()
                .await
                .map_err(|e| RouterError::HandlerError(e.into()))?;
        } else if topic.ends_with("close") {
            info!("Closing blinds");
            self.blinds
                .lock()
                .await
                .close()
                .await
                .map_err(|e| RouterError::HandlerError(e.into()))?;
        }
        Ok(())
    }
}

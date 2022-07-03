use crate::driver::Blinds;
use async_trait::async_trait;
use log::*;
use mqtt_router::{RouteHandler, RouterError};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

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
        } else if topic.ends_with("toggle") {
            info!("Toggling blinds");
            self.blinds
                .lock()
                .await
                .toggle()
                .await
                .map_err(|e| RouterError::HandlerError(e.into()))?;
        } else {
            error!("Unmatched path handler {topic}");
        }
        Ok(())
    }
}

pub struct SwitchHandler {
    blinds: Arc<Mutex<Box<dyn Blinds>>>,
}

impl SwitchHandler {
    pub fn new(blinds: Arc<Mutex<Box<dyn Blinds>>>) -> Box<Self> {
        Box::new(Self { blinds })
    }
}

#[async_trait]
impl RouteHandler for SwitchHandler {
    async fn call(&mut self, _topic: &str, content: &[u8]) -> std::result::Result<(), RouterError> {
        info!("Handling switch data");
        let switch_data: SwitchPayload =
            serde_json::from_slice(content).map_err(|err| RouterError::HandlerError(err.into()))?;

        match switch_data.action {
            Action::Single => {
                info!("Closing blinds");
                self.blinds
                    .lock()
                    .await
                    .close()
                    .await
                    .map_err(|e| RouterError::HandlerError(e.into()))?;
            }
            Action::Long => {
                info!("Opening blinds");
                self.blinds
                    .lock()
                    .await
                    .open()
                    .await
                    .map_err(|e| RouterError::HandlerError(e.into()))?;
            }
            Action::Double => warn!("Double click not supported"),
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Single,
    Double,
    Long,
}

#[derive(Debug, Deserialize)]
pub struct SwitchPayload {
    pub action: Action,
    #[allow(dead_code)]
    pub battery: f32,
    #[allow(dead_code)]
    pub linkquality: f32,
    #[allow(dead_code)]
    pub voltage: f32,
}

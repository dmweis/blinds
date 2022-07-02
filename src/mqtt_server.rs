use super::routes::{BlindsHandler, SwitchHandler};
use crate::{config::MqttConfig, driver::Blinds};
use anyhow::Result;
use log::*;
use mqtt_router::Router;
use rumqttc::{AsyncClient, ConnAck, Event, Incoming, MqttOptions, Publish, QoS, SubscribeFilter};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc::unbounded_channel, Mutex};

enum MqttUpdate {
    Message(Publish),
    Reconnection(ConnAck),
}

pub fn start_mqtt_service(
    blinds: Arc<Mutex<Box<dyn Blinds>>>,
    config: MqttConfig,
) -> anyhow::Result<StatePublisher> {
    let mut mqttoptions =
        MqttOptions::new(&config.client_id, &config.broker_host, config.broker_port);
    info!("Starting MQTT server with options {:?}", mqttoptions);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let base_topic = config.base_route;

    info!("MQTT base topic {}", base_topic);

    let (message_sender, mut message_receiver) = unbounded_channel();

    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(notification) => match notification {
                    Event::Incoming(Incoming::Publish(publish)) => {
                        if let Err(e) = message_sender.send(MqttUpdate::Message(publish)) {
                            eprintln!("Error sending message {}", e);
                        }
                    }
                    Event::Incoming(Incoming::ConnAck(con_ack)) => {
                        if let Err(e) = message_sender.send(MqttUpdate::Reconnection(con_ack)) {
                            eprintln!("Error sending message {}", e);
                        }
                    }
                    _ => (),
                },
                Err(e) => {
                    eprintln!("Error processing eventloop notifications {}", e);
                }
            }
        }
    });

    tokio::spawn({
        let client = client.clone();
        let base_topic = base_topic.clone();
        async move {
            let mut router = Router::default();

            router
                .add_handler(
                    &format!("{base_topic}/#"),
                    BlindsHandler::new(blinds.clone()),
                )
                .unwrap();

            if let Some(switch_topic) = config.switch_topic {
                router
                    .add_handler(&switch_topic, SwitchHandler::new(blinds))
                    .unwrap();
            }

            let topics = router
                .topics_for_subscription()
                .map(|topic| SubscribeFilter {
                    path: topic.to_owned(),
                    qos: QoS::AtMostOnce,
                });
            client.subscribe_many(topics).await.unwrap();

            loop {
                let update = message_receiver.recv().await.unwrap();
                match update {
                    MqttUpdate::Message(message) => {
                        match router
                            .handle_message_ignore_errors(&message.topic, &message.payload)
                            .await
                        {
                            Ok(false) => error!("No handler for topic: \"{}\"", &message.topic),
                            Ok(true) => (),
                            Err(e) => error!("Failed running handler with {:?}", e),
                        }
                    }
                    MqttUpdate::Reconnection(_) => {
                        info!("Reconnecting to broker");
                        let topics =
                            router
                                .topics_for_subscription()
                                .map(|topic| SubscribeFilter {
                                    path: topic.to_owned(),
                                    qos: QoS::AtMostOnce,
                                });
                        client.subscribe_many(topics).await.unwrap();
                    }
                }
            }
        }
    });

    let update_topic = format!("{}/state", base_topic);
    let update_service = StatePublisher::new(client, update_topic);
    Ok(update_service)
}

#[derive(Debug, serde::Serialize, Clone, Copy)]
pub enum BlindsState {
    Open,
    Closed,
    Opening,
    Closing,
    Other,
}

#[derive(Debug, Clone, serde::Serialize)]
struct StateUpdate {
    state: BlindsState,
}

pub struct StatePublisher {
    mqtt: AsyncClient,
    update_topic: String,
}

impl StatePublisher {
    pub fn new(mqtt: AsyncClient, update_topic: String) -> Self {
        Self { mqtt, update_topic }
    }

    pub async fn update_state(&self, new_state: BlindsState) -> Result<()> {
        let update = StateUpdate { state: new_state };
        let json = serde_json::to_vec(&update).unwrap();
        self.mqtt
            .publish(&self.update_topic, QoS::AtMostOnce, false, json)
            .await?;
        Ok(())
    }
}

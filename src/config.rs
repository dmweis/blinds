use crate::{
    driver::{BedroomBlinds, Blinds, LivingRoomBlinds},
    error::DriverError,
};
use anyhow::Result;
use directories::ProjectDirs;
use log::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Serialize, Deserialize)]
pub struct BlindsConfig {
    living_room_blinds: Option<LivingRoomBlindsConfig>,
    bedroom_blinds: Option<BedroomBlindsConfig>,
}

impl Default for BlindsConfig {
    fn default() -> Self {
        let living_room_blinds_config = LivingRoomBlindsConfig {
            ..Default::default()
        };
        Self {
            living_room_blinds: Some(living_room_blinds_config),
            bedroom_blinds: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedroomBlindsConfig {
    pub serial_port: String,
    pub motor_id: u8,
    pub mqtt: MqttConfig,
}

impl Default for BedroomBlindsConfig {
    fn default() -> Self {
        BedroomBlindsConfig {
            serial_port: String::from("/dev/ttyUSB0"),
            motor_id: 1,
            mqtt: MqttConfig::default(),
        }
    }
}

impl BedroomBlindsConfig {
    #[allow(dead_code)]
    // TODO(David): implement bedroom blinds
    pub async fn save(&self, path: &Path) -> Result<()> {
        let config = BlindsConfig {
            bedroom_blinds: Some(self.clone()),
            living_room_blinds: None,
        };
        let contents = serde_yaml::to_vec(&config)?;
        let mut file = File::create(path).await?;
        file.write_all(&contents).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivingRoomBlindsConfig {
    pub serial_port: String,
    pub slide_motor_id: u8,
    pub flip_motor_id: u8,
    pub flip_motor_left: Option<f32>,
    pub flip_motor_right: Option<f32>,
    pub mqtt: MqttConfig,
}

impl Default for LivingRoomBlindsConfig {
    fn default() -> Self {
        LivingRoomBlindsConfig {
            serial_port: String::from("/dev/ttyUSB0"),
            slide_motor_id: 1,
            flip_motor_id: 2,
            flip_motor_left: None,
            flip_motor_right: None,
            mqtt: MqttConfig::default(),
        }
    }
}

impl LivingRoomBlindsConfig {
    pub async fn save(&self, path: &Path) -> Result<()> {
        let config = BlindsConfig {
            living_room_blinds: Some(self.clone()),
            bedroom_blinds: None,
        };
        let contents = serde_yaml::to_vec(&config)?;
        let mut file = File::create(path).await?;
        file.write_all(&contents).await?;
        Ok(())
    }
}

impl BlindsConfig {
    pub async fn load(path: &Path) -> Result<Self> {
        let mut file = File::open(path).await?;
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        Ok(serde_yaml::from_slice(&contents)?)
    }

    pub async fn save(&self, path: &Path) -> Result<()> {
        let contents = serde_yaml::to_vec(self)?;
        let mut file = File::create(path).await?;
        file.write_all(&contents).await?;
        Ok(())
    }

    pub fn default_config_location() -> Option<PathBuf> {
        ProjectDirs::from("com", "dmw", "blinds_app").map(|dirs| dirs.config_dir().to_owned())
    }

    pub async fn driver_from_config(self) -> Result<(Box<dyn Blinds>, MqttConfig)> {
        match (self.living_room_blinds, self.bedroom_blinds) {
            (Some(living_room_blinds), None) => {
                info!("Loading living room blinds mode");
                let mqtt = living_room_blinds.mqtt.clone();
                Ok((
                    Box::new(LivingRoomBlinds::new(living_room_blinds).await?),
                    mqtt,
                ))
            }
            (None, Some(bedroom_blinds)) => {
                info!("Loading bedroom blinds mode");
                let mqtt = bedroom_blinds.mqtt.clone();
                Ok((Box::new(BedroomBlinds::new(bedroom_blinds).await?), mqtt))
            }
            (None, None) => Err(DriverError::MissingRoomConfiguration.into()),
            (_, _) => Err(DriverError::BothRoomConfigsPresent.into()),
        }
    }
}

impl LivingRoomBlindsConfig {
    pub fn flip_motor_center(&self) -> Option<f32> {
        Some((self.flip_motor_left? + self.flip_motor_right?) / 2.0)
    }
}

// weird serde default thing
const DEFAULT_MQTT_PORT: u16 = 1883;

const fn default_mqtt_port() -> u16 {
    DEFAULT_MQTT_PORT
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MqttConfig {
    pub base_route: String,
    pub broker_host: String,
    #[serde(default = "default_mqtt_port")]
    pub broker_port: u16,
    pub client_id: String,
    pub switch_topic: Option<String>,
}

impl Default for MqttConfig {
    fn default() -> Self {
        MqttConfig {
            base_route: "living_room/blinds".to_owned(),
            broker_host: "mqtt".to_owned(),
            broker_port: DEFAULT_MQTT_PORT,
            client_id: "living_room_blinds".to_owned(),
            switch_topic: None,
        }
    }
}

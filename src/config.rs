use crate::{
    driver::{BedroomBlinds, Blinds, LivingRoomBlinds},
    error::DriverError,
};
use anyhow::Result;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct BlindsConfig {
    living_room_blinds: Option<LivingRoomBlindsConfig>,
    bedroom_blinds: Option<BedroomBlindsConfig>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BedroomBlindsConfig {
    pub serial_port: String,
    pub motor_id: u8,
}

impl Default for BedroomBlindsConfig {
    fn default() -> Self {
        BedroomBlindsConfig {
            serial_port: String::from("/dev/ttyUSB0"),
            motor_id: 1,
        }
    }
}

impl BedroomBlindsConfig {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LivingRoomBlindsConfig {
    pub serial_port: String,
    pub slide_motor_id: u8,
    pub flip_motor_id: u8,
    pub flip_motor_left: Option<f32>,
    pub flip_motor_right: Option<f32>,
}

impl Default for LivingRoomBlindsConfig {
    fn default() -> Self {
        LivingRoomBlindsConfig {
            serial_port: String::from("/dev/ttyUSB0"),
            slide_motor_id: 1,
            flip_motor_id: 2,
            flip_motor_left: None,
            flip_motor_right: None,
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

    pub async fn driver_from_config(self) -> Result<Box<dyn Blinds>> {
        match (self.living_room_blinds, self.bedroom_blinds) {
            (Some(living_room_blinds), None) => {
                Ok(Box::new(LivingRoomBlinds::new(living_room_blinds).await?))
            }
            (None, Some(bedroom_blinds)) => Ok(Box::new(BedroomBlinds::new(bedroom_blinds).await?)),
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

use anyhow::Result;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BlindsDriverConfig {
    pub serial_port: String,
    pub slide_motor_id: u8,
    pub flip_motor_id: u8,
    pub flip_motor_left: f32,
    pub flip_motor_right: f32,
}

impl Default for BlindsDriverConfig {
    fn default() -> Self {
        BlindsDriverConfig {
            serial_port: String::from("/dev/ttyUSB0"),
            slide_motor_id: 1,
            flip_motor_id: 2,
            flip_motor_left: std::f32::NAN,
            flip_motor_right: std::f32::NAN,
        }
    }
}

impl BlindsDriverConfig {
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

    pub fn flip_motor_center(&self) -> f32 {
        (self.flip_motor_left + self.flip_motor_right) / 2.0
    }
}

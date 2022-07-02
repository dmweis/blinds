mod bedroom_blinds;
mod living_room_blinds;

use crate::error;
use crate::mqtt_server::StatePublisher;
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;
use std::{path::Path, time::Instant};
use tokio::time::sleep;

pub use bedroom_blinds::BedroomBlinds;
pub use living_room_blinds::LivingRoomBlinds;

const UNCALIBRATED_COLOR: lss_driver::LedColor = lss_driver::LedColor::Magenta;
const CALIBRATED_COLOR: lss_driver::LedColor = lss_driver::LedColor::Off;

const SLIDING_CURRENT_LIMIT: lss_driver::CommandModifier =
    lss_driver::CommandModifier::CurrentLimp(400);

const BEDROOM_LIFTING_CURRENT_LIMIT: lss_driver::CommandModifier =
    lss_driver::CommandModifier::CurrentLimp(600);

const SLIDING_SPEED: f32 = 340.0;

const LIVING_ROOM_SLIDING_TIMEOUT: Duration = Duration::from_secs(22);
const LIVING_ROOM_FLIPPER_TIMEOUT: Duration = Duration::from_secs(3);
const BEDROOM_SLIDING_TIMEOUT: Duration = Duration::from_secs(20);

const BEDROOM_DOOR_TOP_OFFSET: f32 = 100.0;
const BEDROOM_BLIND_BOTTOM_OFFSET: f32 = 4500.0;

#[derive(Debug, serde::Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum BlindsState {
    Open,
    Closed,
    Opening,
    Closing,
    Other,
}

#[async_trait]
pub trait Blinds: Send {
    async fn open(&mut self) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
    async fn were_motors_rebooted(&mut self) -> Result<bool>;
    async fn calibrate(&mut self, config_path: &Path) -> Result<()>;
    fn needs_calibration(&self) -> bool;
    fn set_state_publisher(&mut self, state_publisher: StatePublisher);
}

pub async fn wait_until_motor_stopped(
    driver: &mut lss_driver::LSSDriver,
    id: u8,
    timeout: Duration,
) -> Result<()> {
    let start_time = Instant::now();
    sleep(Duration::from_secs(1)).await;
    loop {
        if start_time.elapsed() > timeout {
            if driver.limp(id).await.is_err() {
                error!("Failed to stop motor after timeout");
            }
            error!(
                "Timed out waiting for stop {}ms motor {}",
                timeout.as_millis(),
                id
            );
            return Err(error::DriverError::WaitingForStopTimedOut.into());
        }
        let status = driver.query_status(id).await?;
        match status {
            lss_driver::MotorStatus::Limp | lss_driver::MotorStatus::Holding => return Ok(()),
            lss_driver::MotorStatus::Unknown
            | lss_driver::MotorStatus::OutsideLimits
            | lss_driver::MotorStatus::Stuck
            | lss_driver::MotorStatus::Blocked
            | lss_driver::MotorStatus::SafeMode => {
                return Err(error::DriverError::BadMotorStatus(status).into())
            }
            _ => (),
        }
        sleep(Duration::from_millis(50)).await;
    }
}

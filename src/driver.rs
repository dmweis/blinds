use crate::{
    config::{BedroomBlindsConfig, LivingRoomBlindsConfig},
    error,
};
use anyhow::Result;
use async_trait::async_trait;
use log::*;
use std::time::Duration;
use std::{path::Path, time::Instant};
use tokio::time::sleep;

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
const BEDROOM_BLIND_BOTTOM_OFFSET: f32 = 4400.0;

#[async_trait]
pub trait Blinds: Send {
    async fn open(&mut self) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
    async fn were_motors_rebooted(&mut self) -> Result<bool>;
    async fn calibrate(&mut self, config_path: &Path) -> Result<()>;
    fn needs_calibration(&self) -> bool;
}

pub struct LivingRoomBlinds {
    pub config: LivingRoomBlindsConfig,
    driver: lss_driver::LSSDriver,
}

pub struct BedroomBlinds {
    pub config: BedroomBlindsConfig,
    driver: lss_driver::LSSDriver,
}

impl BedroomBlinds {
    pub async fn new(config: BedroomBlindsConfig) -> Result<Self> {
        let mut serial_driver = lss_driver::LSSDriver::new(&config.serial_port)?;
        serial_driver.limp(lss_driver::BROADCAST_ID).await?;
        Ok(Self {
            config,
            driver: serial_driver,
        })
    }

    async fn configure(&mut self) -> Result<()> {
        self.driver
            .configure_color(self.config.motor_id, UNCALIBRATED_COLOR)
            .await?;
        self.driver
            .set_color(self.config.motor_id, CALIBRATED_COLOR)
            .await?;
        Ok(())
    }

    async fn open_until_limit(&mut self) -> Result<()> {
        self.driver
            .set_rotation_speed_with_modifier(
                self.config.motor_id,
                -SLIDING_SPEED,
                BEDROOM_LIFTING_CURRENT_LIMIT,
            )
            .await?;
        wait_until_motor_stopped(
            &mut self.driver,
            self.config.motor_id,
            BEDROOM_SLIDING_TIMEOUT,
        )
        .await?;
        self.driver.limp(self.config.motor_id).await?;
        Ok(())
    }
}

impl LivingRoomBlinds {
    pub async fn new(config: LivingRoomBlindsConfig) -> Result<Self> {
        let mut serial_driver = lss_driver::LSSDriver::new(&config.serial_port)?;
        serial_driver.limp(lss_driver::BROADCAST_ID).await?;
        Ok(Self {
            config,
            driver: serial_driver,
        })
    }

    #[allow(dead_code)]
    // TODO(David): I think this is not needed yet
    pub async fn reset_motors(&mut self) -> Result<()> {
        self.driver.reset(lss_driver::BROADCAST_ID).await?;
        sleep(Duration::from_secs(2)).await;
        Ok(())
    }

    async fn configure(&mut self) -> Result<()> {
        self.driver
            .configure_color(lss_driver::BROADCAST_ID, UNCALIBRATED_COLOR)
            .await?;
        self.driver
            .set_color(lss_driver::BROADCAST_ID, CALIBRATED_COLOR)
            .await?;
        Ok(())
    }

    pub async fn flip_open(&mut self) -> Result<()> {
        self.driver
            .move_to_position_with_modifier(
                self.config.flip_motor_id,
                self.config
                    .flip_motor_center()
                    .ok_or(error::DriverError::MissingMotorConfig)?,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        wait_until_motor_stopped(
            &mut self.driver,
            self.config.flip_motor_id,
            LIVING_ROOM_FLIPPER_TIMEOUT,
        )
        .await?;
        self.driver.limp(self.config.flip_motor_id).await?;
        Ok(())
    }

    pub async fn flip_close_left(&mut self) -> Result<()> {
        self.driver
            .move_to_position_with_modifier(
                self.config.flip_motor_id,
                self.config
                    .flip_motor_left
                    .ok_or(error::DriverError::MissingMotorConfig)?,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        wait_until_motor_stopped(
            &mut self.driver,
            self.config.flip_motor_id,
            LIVING_ROOM_FLIPPER_TIMEOUT,
        )
        .await?;
        self.driver.limp(self.config.flip_motor_id).await?;
        Ok(())
    }

    #[allow(dead_code)]
    // TODO(David): I don't think I want to do this almost ever
    pub async fn flip_close_right(&mut self) -> Result<()> {
        self.driver
            .move_to_position_with_modifier(
                self.config.flip_motor_id,
                self.config
                    .flip_motor_right
                    .ok_or(error::DriverError::MissingMotorConfig)?,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        wait_until_motor_stopped(
            &mut self.driver,
            self.config.flip_motor_id,
            LIVING_ROOM_FLIPPER_TIMEOUT,
        )
        .await?;
        self.driver.limp(self.config.flip_motor_id).await?;
        Ok(())
    }

    pub async fn slide_open(&mut self) -> Result<()> {
        self.driver
            .set_rotation_speed_with_modifier(
                self.config.slide_motor_id,
                -SLIDING_SPEED,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        wait_until_motor_stopped(
            &mut self.driver,
            self.config.slide_motor_id,
            LIVING_ROOM_SLIDING_TIMEOUT,
        )
        .await?;
        self.driver.limp(self.config.slide_motor_id).await?;
        Ok(())
    }

    pub async fn slide_closed(&mut self) -> Result<()> {
        self.driver
            .set_rotation_speed_with_modifier(
                self.config.slide_motor_id,
                SLIDING_SPEED,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        wait_until_motor_stopped(
            &mut self.driver,
            self.config.slide_motor_id,
            LIVING_ROOM_SLIDING_TIMEOUT,
        )
        .await?;
        self.driver.limp(self.config.slide_motor_id).await?;
        Ok(())
    }

    pub async fn calibrate_flipper(&mut self) -> Result<()> {
        let start_color = self.driver.query_color(self.config.flip_motor_id).await?;
        let start_pose = self
            .driver
            .query_position(self.config.flip_motor_id)
            .await?;
        let mut left = start_pose;
        let mut right = start_pose;

        // wait for motor to start moving
        let move_detection_start = std::time::Instant::now();
        info!("Waiting for moving to start");
        self.driver
            .set_color(self.config.flip_motor_id, lss_driver::LedColor::Yellow)
            .await?;
        loop {
            let current_pose = self
                .driver
                .query_position(self.config.flip_motor_id)
                .await?;
            if (start_pose - current_pose).abs() > 20.0 {
                info!("Detected started moving");
                break;
            }
            // blink
            if move_detection_start.elapsed().as_secs() % 2 == 0 {
                self.driver
                    .set_color(self.config.flip_motor_id, lss_driver::LedColor::Yellow)
                    .await?;
            } else {
                self.driver
                    .set_color(self.config.flip_motor_id, lss_driver::LedColor::Off)
                    .await?;
            }
            sleep(Duration::from_millis(100)).await;
        }

        // Moving start
        // Run calibration loop
        self.driver
            .set_color(self.config.flip_motor_id, lss_driver::LedColor::Green)
            .await?;
        let detection_loop_start = std::time::Instant::now();
        while detection_loop_start.elapsed() < Duration::from_secs(20) {
            let current_pose = self
                .driver
                .query_position(self.config.flip_motor_id)
                .await?;
            left = left.min(current_pose);
            right = right.max(current_pose);
            sleep(Duration::from_millis(20)).await;
        }
        self.driver
            .set_color(self.config.flip_motor_id, lss_driver::LedColor::Green)
            .await?;
        info!("Finished calibration");
        info!("left: {}, right: {}", left, right);
        self.config.flip_motor_left = Some(left);
        self.config.flip_motor_right = Some(right);
        self.driver
            .set_color(self.config.flip_motor_id, start_color)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl Blinds for LivingRoomBlinds {
    async fn were_motors_rebooted(&mut self) -> Result<bool> {
        let flip_motor_rebooted =
            self.driver.query_color(self.config.flip_motor_id).await? != CALIBRATED_COLOR;
        let slide_motor_rebooted =
            self.driver.query_color(self.config.slide_motor_id).await? != CALIBRATED_COLOR;
        Ok(flip_motor_rebooted || slide_motor_rebooted)
    }

    async fn open(&mut self) -> Result<()> {
        self.flip_open().await?;
        self.slide_open().await?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.flip_open().await?;
        self.slide_closed().await?;
        self.flip_close_left().await?;
        Ok(())
    }

    async fn calibrate(&mut self, config_path: &Path) -> Result<()> {
        info!("Starting calibration for living room blinds");
        self.calibrate_flipper().await?;
        self.flip_open().await?;
        sleep(Duration::from_secs(2)).await;
        self.flip_close_left().await?;
        self.config.save(config_path).await?;
        self.configure().await?;
        Ok(())
    }

    fn needs_calibration(&self) -> bool {
        self.config.flip_motor_left.is_none() || self.config.flip_motor_right.is_none()
    }
}

#[async_trait]
impl Blinds for BedroomBlinds {
    async fn were_motors_rebooted(&mut self) -> Result<bool> {
        let motor_rebooted =
            self.driver.query_color(self.config.motor_id).await? != CALIBRATED_COLOR;
        Ok(motor_rebooted)
    }

    async fn open(&mut self) -> Result<()> {
        // make sure speed is limited
        self.driver
            .set_maximum_speed(self.config.motor_id, SLIDING_SPEED)
            .await?;
        self.driver
            .move_to_position_with_modifier(
                self.config.motor_id,
                self.config
                    .top_position
                    .ok_or(error::DriverError::MissingMotorConfig)?,
                BEDROOM_LIFTING_CURRENT_LIMIT,
            )
            .await?;
        wait_until_motor_stopped(
            &mut self.driver,
            self.config.motor_id,
            BEDROOM_SLIDING_TIMEOUT,
        )
        .await?;
        self.driver.limp(self.config.motor_id).await?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        // make sure speed is limited
        self.driver
            .set_maximum_speed(self.config.motor_id, SLIDING_SPEED)
            .await?;
        self.driver
            .move_to_position_with_modifier(
                self.config.motor_id,
                self.config
                    .top_position
                    .ok_or(error::DriverError::MissingMotorConfig)?
                    + BEDROOM_BLIND_BOTTOM_OFFSET,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        wait_until_motor_stopped(
            &mut self.driver,
            self.config.motor_id,
            BEDROOM_SLIDING_TIMEOUT,
        )
        .await?;
        self.driver.limp(self.config.motor_id).await?;
        Ok(())
    }

    async fn calibrate(&mut self, config_path: &Path) -> Result<()> {
        info!("Starting calibration for bedroom blinds");
        self.open_until_limit().await?;
        // top of bedroom is a bit away from the place where we stop for current limit
        let top_position =
            self.driver.query_position(self.config.motor_id).await? + BEDROOM_DOOR_TOP_OFFSET;
        self.config.top_position = Some(top_position);
        self.config.save(config_path).await?;
        self.configure().await?;
        Ok(())
    }

    fn needs_calibration(&self) -> bool {
        self.config.top_position.is_none()
    }
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

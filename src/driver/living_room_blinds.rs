use super::{
    wait_until_motor_stopped, Blinds, BlindsState, CALIBRATED_COLOR, LIVING_ROOM_FLIPPER_TIMEOUT,
    LIVING_ROOM_SLIDING_TIMEOUT, SLIDING_CURRENT_LIMIT, SLIDING_SPEED, UNCALIBRATED_COLOR,
};
use crate::{config::LivingRoomBlindsConfig, error, mqtt_server::StatePublisher};
use anyhow::Result;
use async_trait::async_trait;
use log::*;
use std::{path::Path, time::Duration};
use tokio::time::sleep;

pub struct LivingRoomBlinds {
    pub config: LivingRoomBlindsConfig,
    driver: lss_driver::LSSDriver,
    state_publisher: Option<StatePublisher>,
    state: BlindsState,
}

impl LivingRoomBlinds {
    pub async fn new(config: LivingRoomBlindsConfig) -> Result<Self> {
        let mut serial_driver = lss_driver::LSSDriver::new(&config.serial_port)?;
        serial_driver.limp(lss_driver::BROADCAST_ID).await?;
        Ok(Self {
            config,
            driver: serial_driver,
            state_publisher: None,
            state: BlindsState::Other,
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
        let flip_motor_center = self
            .config
            .flip_motor_center()
            .ok_or(error::DriverError::MissingMotorConfig)?;

        let current_position = self
            .driver
            .query_position(self.config.flip_motor_id)
            .await?;

        let delta = (current_position - flip_motor_center).abs();
        if delta < 3.0 {
            info!(
                "Flip motor already opened pose {} center {}",
                current_position, flip_motor_center
            );
            self.driver.limp(self.config.flip_motor_id).await?;
            return Ok(());
        }

        self.driver
            .move_to_position_with_modifier(
                self.config.flip_motor_id,
                flip_motor_center,
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

    pub async fn flip_partial_left(&mut self, open: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&open) {
            error!("Open has to be between 0.0 and 1.0, got {}", open);
            return Err(error::DriverError::PartialPositionOutOfRange.into());
        }

        let flip_motor_center = self
            .config
            .flip_motor_center()
            .ok_or(error::DriverError::MissingMotorConfig)?;

        let fully_closed = self
            .config
            .flip_motor_left
            .ok_or(error::DriverError::MissingMotorConfig)?;

        let desired_position = fully_closed + open * (flip_motor_center - fully_closed);

        let current_position = self
            .driver
            .query_position(self.config.flip_motor_id)
            .await?;

        let delta = (current_position - desired_position).abs();
        if delta < 3.0 {
            info!(
                "Flip motor already partially opened current pose {} desired {}",
                current_position, desired_position
            );
            self.driver.limp(self.config.flip_motor_id).await?;
            return Ok(());
        }

        self.driver
            .move_to_position_with_modifier(
                self.config.flip_motor_id,
                desired_position,
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

    async fn set_state(&mut self, state: BlindsState) -> Result<()> {
        self.state = state;
        if let Some(ref state_publisher) = self.state_publisher {
            state_publisher.update_state(state).await?;
        }
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
        if matches!(self.state, BlindsState::Open) {
            info!("Blinds already open");
            return Ok(());
        }
        self.set_state(BlindsState::Opening).await?;
        self.flip_open().await?;
        self.slide_open().await?;
        self.set_state(BlindsState::Open).await?;
        Ok(())
    }

    async fn partial_open(&mut self, open: f32) -> Result<()> {
        self.set_state(BlindsState::Opening).await?;
        match self.state {
            BlindsState::Open => {
                self.flip_open().await?;
                self.slide_closed().await?;
            }
            BlindsState::Closed => {
                self.flip_open().await?;
            }
            _ => (),
        }
        self.flip_partial_left(open).await?;
        self.set_state(BlindsState::Partial).await?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        if matches!(self.state, BlindsState::Closed) {
            info!("Blinds already closed");
            return Ok(());
        }
        self.set_state(BlindsState::Closing).await?;
        self.flip_open().await?;
        self.slide_closed().await?;
        self.flip_close_left().await?;
        self.set_state(BlindsState::Closed).await?;
        Ok(())
    }

    async fn toggle(&mut self) -> Result<()> {
        info!("Toggling blinds");
        match self.state {
            BlindsState::Closed | BlindsState::Closing => self.open().await?,
            BlindsState::Open
            | BlindsState::Opening
            | BlindsState::Other
            | BlindsState::Partial => self.close().await?,
        }
        Ok(())
    }

    async fn calibrate(&mut self, config_path: &Path) -> Result<()> {
        self.set_state(BlindsState::Other).await?;
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

    fn set_state_publisher(&mut self, state_publisher: StatePublisher) {
        self.state_publisher = Some(state_publisher)
    }
}

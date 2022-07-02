use super::{
    wait_until_motor_stopped, Blinds, BEDROOM_BLIND_BOTTOM_OFFSET, BEDROOM_DOOR_TOP_OFFSET,
    BEDROOM_LIFTING_CURRENT_LIMIT, BEDROOM_SLIDING_TIMEOUT, CALIBRATED_COLOR,
    SLIDING_CURRENT_LIMIT, SLIDING_SPEED, UNCALIBRATED_COLOR,
};
use crate::{config::BedroomBlindsConfig, error};
use anyhow::Result;
use async_trait::async_trait;
use log::*;
use std::path::Path;

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
        // top of bedroom is a bit away from the place where we stop for current limit
        self.driver
            .move_to_position_with_modifier(
                self.config.motor_id,
                self.config
                    .top_position
                    .ok_or(error::DriverError::MissingMotorConfig)?
                    + BEDROOM_DOOR_TOP_OFFSET,
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
        let top_position = self.driver.query_position(self.config.motor_id).await?;
        self.config.top_position = Some(top_position);
        self.config.save(config_path).await?;
        self.configure().await?;
        self.open().await?;
        Ok(())
    }

    fn needs_calibration(&self) -> bool {
        self.config.top_position.is_none()
    }
}

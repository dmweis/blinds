mod config;
mod error;

use anyhow::Result;
use clap::Parser;
use config::BlindsDriverConfig;
use log::*;
use std::{path::PathBuf, time::Duration};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// path to config file
    #[clap(long)]
    config: Option<PathBuf>,
    /// create default config
    #[clap(long)]
    create_default_config: bool,

    /// start with calibration
    #[clap(long)]
    run_calibration: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stdout,
        simplelog::ColorChoice::Auto,
    )?;

    let config_path = args
        .config
        .unwrap_or_else(|| BlindsDriverConfig::default_config_location().unwrap());

    if args.create_default_config {
        BlindsDriverConfig::default().save(&config_path).await?;
    }

    let config = BlindsDriverConfig::load(&config_path).await?;

    let mut driver = BlindsDriver::new(config).await?;

    let were_motors_rebooted = driver.were_motors_rebooted().await?;

    if args.run_calibration || were_motors_rebooted {
        if were_motors_rebooted {
            warn!("Motors seems to have been rebooted since the last run.");
        }
        info!("Starting calibration");
        driver.calibrate_flipper().await?;
        driver.flip_open().await?;
        sleep(Duration::from_secs(2)).await;
        driver.flip_close_left().await?;
        driver.config.save(&config_path).await?;
        driver.configure().await?;
    }

    if driver.were_motors_rebooted().await? {
        info!("Motors were rebooted");
    } else {
        info!("Motors were NOT rebooted");
    }

    driver.open().await?;
    sleep(Duration::from_secs(2)).await;
    driver.close().await?;

    Ok(())
}

struct BlindsDriver {
    config: BlindsDriverConfig,
    driver: lss_driver::LSSDriver,
}

const UNCALIBRATED_COLOR: lss_driver::LedColor = lss_driver::LedColor::Magenta;
const CALIBRATED_COLOR: lss_driver::LedColor = lss_driver::LedColor::Cyan;
const SLIDING_CURRENT_LIMIT: lss_driver::CommandModifier =
    lss_driver::CommandModifier::CurrentLimp(400);
const SLIDING_SPEED: f32 = 340.0;

impl BlindsDriver {
    async fn new(config: BlindsDriverConfig) -> Result<Self> {
        let serial_driver = lss_driver::LSSDriver::new(&config.serial_port)?;
        Ok(Self {
            config,
            driver: serial_driver,
        })
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

    async fn reset_motors(&mut self) -> Result<()> {
        self.driver.reset(lss_driver::BROADCAST_ID).await?;
        sleep(Duration::from_secs(2)).await;
        Ok(())
    }

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

    async fn flip_open(&mut self) -> Result<()> {
        self.driver
            .move_to_position_with_modifier(
                self.config.flip_motor_id,
                self.config.flip_motor_center(),
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        self.wait_until_stopped(self.config.flip_motor_id).await?;
        self.driver.limp(self.config.flip_motor_id).await?;
        Ok(())
    }

    async fn flip_close_left(&mut self) -> Result<()> {
        self.driver
            .move_to_position_with_modifier(
                self.config.flip_motor_id,
                self.config.flip_motor_left,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        self.wait_until_stopped(self.config.flip_motor_id).await?;
        self.driver.limp(self.config.flip_motor_id).await?;
        Ok(())
    }

    async fn flip_close_right(&mut self) -> Result<()> {
        self.driver
            .move_to_position_with_modifier(
                self.config.flip_motor_id,
                self.config.flip_motor_right,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        self.wait_until_stopped(self.config.flip_motor_id).await?;
        self.driver.limp(self.config.flip_motor_id).await?;
        Ok(())
    }

    async fn slide_open(&mut self) -> Result<()> {
        self.driver
            .set_rotation_speed_with_modifier(
                self.config.slide_motor_id,
                -SLIDING_SPEED,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        self.wait_until_stopped(self.config.slide_motor_id).await?;
        self.driver.limp(self.config.slide_motor_id).await?;
        Ok(())
    }

    async fn slide_closed(&mut self) -> Result<()> {
        self.driver
            .set_rotation_speed_with_modifier(
                self.config.slide_motor_id,
                SLIDING_SPEED,
                SLIDING_CURRENT_LIMIT,
            )
            .await?;
        self.wait_until_stopped(self.config.slide_motor_id).await?;
        self.driver.limp(self.config.slide_motor_id).await?;
        Ok(())
    }

    async fn wait_until_stopped(&mut self, id: u8) -> Result<()> {
        sleep(Duration::from_secs(1)).await;
        loop {
            let status = self.driver.query_status(id).await?;
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

    async fn calibrate_flipper(&mut self) -> Result<()> {
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
        self.config.flip_motor_left = left;
        self.config.flip_motor_right = right;
        self.driver
            .set_color(self.config.flip_motor_id, start_color)
            .await?;
        Ok(())
    }
}

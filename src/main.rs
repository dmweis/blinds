mod config;
mod driver;
mod error;

use anyhow::Result;
use clap::Parser;
use config::BlindsDriverConfig;
use driver::BlindsDriver;
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

#[cfg(test)]
mod test {
    #[test]
    fn empty_test() {}
}

mod config;
mod driver;
mod error;

use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use clap::Parser;
use config::BlindsDriverConfig;
use driver::BlindsDriver;
use log::*;
use std::{path::PathBuf, time::Duration};
use tokio::sync::Mutex;
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

#[post("/open_blinds")]
async fn open_blinds_handler(driver: web::Data<Mutex<BlindsDriver>>) -> impl Responder {
    let mut driver = driver.lock().await;
    if driver.open().await.is_ok() {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

#[post("/close_blinds")]
async fn close_blinds_handler(driver: web::Data<Mutex<BlindsDriver>>) -> impl Responder {
    let mut driver = driver.lock().await;
    if driver.close().await.is_ok() {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::InternalServerError().finish()
    }
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

    let address = format!("{}:{}", "0.0.0.0", 8080);
    let driver = web::Data::new(Mutex::new(driver));

    HttpServer::new(move || {
        let driver = driver.clone();
        App::new()
            .service(open_blinds_handler)
            .service(close_blinds_handler)
            .app_data(driver)
    })
    .bind(address)?
    .run()
    .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn empty_test() {}
}

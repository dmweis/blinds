mod config;
mod driver;
mod error;
mod mqtt;

use actix_web::{middleware::Logger, post, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use clap::Parser;
use config::BlindsConfig;
use driver::Blinds;
use log::*;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

use crate::mqtt::start_mqtt_service;

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
async fn open_blinds_handler(driver: web::Data<Mutex<Box<dyn Blinds>>>) -> impl Responder {
    let mut driver = driver.lock().await;
    if let Err(e) = driver.open().await {
        error!("Error while opening blinds {e}");
        HttpResponse::InternalServerError().finish()
    } else {
        HttpResponse::Ok().finish()
    }
}

#[post("/close_blinds")]
async fn close_blinds_handler(driver: web::Data<Mutex<Box<dyn Blinds>>>) -> impl Responder {
    let mut driver = driver.lock().await;
    if let Err(e) = driver.close().await {
        error!("Error while closing blinds {e}");
        HttpResponse::InternalServerError().finish()
    } else {
        HttpResponse::Ok().finish()
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

    info!("Starting blinds");

    let config_path = args
        .config
        .unwrap_or_else(|| BlindsConfig::default_config_location().unwrap());

    let mut new_config = false;
    if args.create_default_config {
        BlindsConfig::default().save(&config_path).await?;
        new_config = true;
    } else if !config_path.exists() {
        BlindsConfig::default().save(&config_path).await?;
        new_config = true;
    }

    let config = BlindsConfig::load(&config_path).await?;

    let (mut driver, mqtt_config) = config.driver_from_config().await?;

    let were_motors_rebooted = driver.were_motors_rebooted().await?;
    if new_config || args.run_calibration || were_motors_rebooted {
        if were_motors_rebooted {
            warn!("Motors seems to have been rebooted since the last run.");
        }
        if new_config {
            warn!("Fresh config written. Running calibration.");
        }
        driver.calibrate(&config_path).await?;
    }

    let address = format!("{}:{}", "0.0.0.0", 8080);
    info!("Binding on address: {address}");
    let driver = Arc::new(Mutex::new(driver));

    start_mqtt_service(driver.clone(), mqtt_config).expect("Failed to start mqtt server");

    let driver = web::Data::from(driver);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::new("%r %s %U"))
            .service(open_blinds_handler)
            .service(close_blinds_handler)
            .app_data(driver.clone())
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

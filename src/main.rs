mod config;
mod driver;
mod error;

use actix_web::{middleware::Logger, post, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use clap::Parser;
use config::BlindsConfig;
use driver::{Blinds, LivingRoomBlinds};
use log::*;
use std::path::PathBuf;
use tokio::sync::Mutex;

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
async fn open_blinds_handler(driver: web::Data<Mutex<LivingRoomBlinds>>) -> impl Responder {
    let mut driver = driver.lock().await;
    if let Err(e) = driver.open().await {
        error!("Error while opening blinds {e}");
        HttpResponse::InternalServerError().finish()
    } else {
        HttpResponse::Ok().finish()
    }
}

#[post("/close_blinds")]
async fn close_blinds_handler(driver: web::Data<Mutex<LivingRoomBlinds>>) -> impl Responder {
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

    if args.create_default_config {
        BlindsConfig::default().save(&config_path).await?;
    }

    let config = BlindsConfig::load(&config_path).await?;

    let mut driver = config.driver_from_config().await?;

    let were_motors_rebooted = driver.were_motors_rebooted().await?;
    if args.run_calibration || were_motors_rebooted {
        if were_motors_rebooted {
            warn!("Motors seems to have been rebooted since the last run.");
        }
        driver.calibrate(&config_path).await?;
    }

    let address = format!("{}:{}", "0.0.0.0", 8080);
    info!("Binding on address: {address}");
    let driver = web::Data::new(Mutex::new(driver));

    HttpServer::new(move || {
        let driver = driver.clone();
        App::new()
            .wrap(Logger::new("%r %s %U"))
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

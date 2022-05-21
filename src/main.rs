use anyhow::Result;
use clap::Parser;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Serial port to use
    port: String,
    /// mode
    #[clap(long)]
    sideways: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut driver = lss_driver::LSSDriver::new(&args.port).unwrap();
    driver
        .set_color(lss_driver::BROADCAST_ID, lss_driver::LedColor::Magenta)
        .await?;

    let a = driver.query_position(1).await?;
    println!("Sideway motor is at {}", a);
    driver
        .set_maximum_speed(lss_driver::BROADCAST_ID, 20.0)
        .await?;

    if args.sideways {
        loop {
            let speed = 340.0;
            wait_and_print_current(&mut driver).await?;
            driver
                .set_rotation_speed_with_modifier(
                    1,
                    speed,
                    lss_driver::CommandModifier::CurrentLimp(400),
                )
                .await?;
            wait_and_print_current(&mut driver).await?;
            driver.limp(1).await?;
            wait_and_print_current(&mut driver).await?;
            driver
                .set_rotation_speed_with_modifier(
                    1,
                    -speed,
                    lss_driver::CommandModifier::CurrentLimp(400),
                )
                .await?;
            wait_and_print_current(&mut driver).await?;
            driver.limp(1).await?;
        }
    } else {
        loop {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
            flip_open(&mut driver).await?;
            std::io::stdin().read_line(&mut line).unwrap();
            close_towards_me(&mut driver).await?;
            std::io::stdin().read_line(&mut line).unwrap();
            close_away_from_me(&mut driver).await?;
        }
    }
}

async fn wait_and_print_current(driver: &mut lss_driver::LSSDriver) -> Result<()> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    let a = driver.query_current(1).await?;
    driver.limp(1).await?;
    println!("Current is {}", a);
    Ok(())
}

async fn wait_and_print_position(driver: &mut lss_driver::LSSDriver) -> Result<()> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    let a = driver.query_position(1).await?;
    println!("Motor position is {}", a);
    Ok(())
}

async fn shut(driver: &mut lss_driver::LSSDriver) -> Result<()> {
    driver
        .move_to_position_with_modifier(1, 4500.0, lss_driver::CommandModifier::CurrentLimp(400))
        .await?;

    sleep(Duration::from_secs(5)).await;
    Ok(())
}

async fn open(driver: &mut lss_driver::LSSDriver) -> Result<()> {
    driver
        .move_to_position_with_modifier(1, 0.0, lss_driver::CommandModifier::CurrentLimp(400))
        .await?;
    sleep(Duration::from_secs(5)).await;
    Ok(())
}

const FLIPPER_CENTER: f32 = -100.0;

async fn flip_open(driver: &mut lss_driver::LSSDriver) -> Result<()> {
    driver.move_to_position(2, FLIPPER_CENTER).await?;
    Ok(())
}

async fn close_towards_me(driver: &mut lss_driver::LSSDriver) -> Result<()> {
    driver.move_to_position(2, FLIPPER_CENTER + 500.0).await?;
    Ok(())
}

async fn close_away_from_me(driver: &mut lss_driver::LSSDriver) -> Result<()> {
    driver.move_to_position(2, FLIPPER_CENTER - 500.0).await?;
    Ok(())
}

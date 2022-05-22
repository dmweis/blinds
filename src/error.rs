use lss_driver::MotorStatus;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriverError {
    #[error("bad motor status {0:?}")]
    BadMotorStatus(MotorStatus),
}

use lss_driver::MotorStatus;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriverError {
    #[error("bad motor status {0:?}")]
    BadMotorStatus(MotorStatus),
    #[error("missing motor config")]
    MissingMotorConfig,
    #[error("missing room configuration")]
    MissingRoomConfiguration,
    #[error("found both room configurations")]
    BothRoomConfigsPresent,
    #[error("waiting for stop timed out")]
    WaitingForStopTimedOut,
    #[error("partial position out of range")]
    PartialPositionOutOfRange,
}

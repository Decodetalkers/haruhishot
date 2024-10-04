use std::io;

use thiserror::Error;
/// Error
/// it describe three kind of error
/// 1. failed when init
/// 2. failed in queue
/// 3. failed in shm copy
#[derive(Error, Debug)]
pub enum HaruhiError {
    #[error("Init Failed")]
    InitFailed(String),
    #[error("Error during queue")]
    QueueError(String),
    #[error("Error in write image in shm")]
    ShmError(#[from] io::Error),
    #[error("Not Support format")]
    NotSupportFormat
}

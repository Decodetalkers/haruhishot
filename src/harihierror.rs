use thiserror::Error;
/// Error
/// it describe three kind of error
/// 1. failed when init
/// 2. failed in queue
/// 3. failed in pipereader
#[derive(Error, Debug)]
pub enum HarihiError {
    #[error("Init Failed")]
    InitFailed(String),
    #[error("Error during queue")]
    QueueError(String),
}

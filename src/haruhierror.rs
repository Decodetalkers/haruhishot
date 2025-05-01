use std::io;
use thiserror::Error;
use wayland_client::{
    ConnectError, DispatchError,
    globals::{BindError, GlobalError},
};
/// Error
/// it describe three kind of error
/// 1. failed when init
/// 2. failed in queue
/// 3. failed in shm copy
#[derive(Error, Debug)]
pub enum HaruhiError {
    #[error("Init Failed connection")]
    InitFailedConnection(#[from] ConnectError),
    #[error("Init Failed Global")]
    InitFailedGlobal(#[from] GlobalError),
    #[error("Dispatch Error")]
    DispatchError(#[from] DispatchError),
    #[error("Error during queue")]
    BindError(#[from] BindError),
    #[error("Error in write image in shm")]
    ShmError(#[from] io::Error),
    #[error("Not Support format")]
    NotSupportFormat,
}

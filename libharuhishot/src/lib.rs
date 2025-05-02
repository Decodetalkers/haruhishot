mod convert;
mod haruhierror;
mod overlay;
mod screenshot;
mod state;
mod utils;

pub use screenshot::{ImageClipInfo, ImageInfo};
pub use state::*;
pub use utils::*;

pub use image::ColorType;

pub use haruhierror::HaruhiError as Error;

/// for user to read the state, report some object
pub mod reexport {
    pub mod wl_output {
        /// rexport wl_output Transform
        pub use wayland_client::protocol::wl_output::Transform;
        pub use wayland_client::protocol::wl_output::WlOutput;
    }
    pub mod wl_shm {
        /// reexport wl_shm Format
        pub use wayland_client::protocol::wl_shm::Format;
    }
    /// rexport wl_output Transform
    pub use wl_output::Transform;
    /// reexport wl_shm Format
    pub use wl_shm::Format;
}

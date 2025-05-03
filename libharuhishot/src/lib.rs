mod convert;
mod haruhierror;
mod overlay;
mod screenshot;
mod state;
mod utils;

pub use screenshot::{AreaSelectCallback, CaptureOption, ImageInfo, ImageViewInfo};
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

    pub mod ext_foreign_toplevel_handle_v1 {
        pub use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;
    }
}

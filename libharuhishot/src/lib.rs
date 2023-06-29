pub mod haruhierror;
pub mod wlrcopystate;
pub mod wlrshotbasestate;

pub use wlrcopystate::{FrameFormat, FrameInfo};
pub use wlrshotbasestate::HaruhiShotState;

/// for user to read the state, report some object
pub mod reexport {
    pub mod wl_output {
        /// rexport wl_output Transform
        pub use wayland_client::protocol::wl_output::Transform;
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

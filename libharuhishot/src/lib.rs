pub mod harihierror;
pub mod wlrcopystate;
pub mod wlrshotbasestate;

pub use wlrcopystate::{FrameFormat, FrameInfo};
pub use wlrshotbasestate::HarihiShotState;

/// rexport wl_output Transform
/// reexport wl_shm Format
/// for user to read the state
pub mod reexport {
    pub mod wl_output {
        pub use wayland_client::protocol::wl_output::Transform;
    }
    pub mod wl_shm {
        pub use wayland_client::protocol::wl_shm::Format;
    }
    pub use wl_output::Transform;
    pub use wl_shm::Format;
}

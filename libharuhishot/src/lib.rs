//!
//! libharuhishot, it is used for wlr-screencopy, split it because I want to help with wayshot, but
//! I also learn a lot. I like my program very much, because it makes me feel alive. Wayshot is a
//! good program, please help them.
//!
//! The lib is simple enough to use, you can take the haruhishot for example, simple useage is like
//!
//! ```rust, no_run
//! use libharuhishot::HaruhiShotState;
//! let mut state = HaruhiShotState::init().unwrap();
//! let buffer = state.capture_output_frame(
//!     &state.displays[0].clone(),
//!     state.display_logic_size[0],
//!     state.display_transform[0],
//!     None
//! ).unwrap();
//!
//! ```
//! Then you will get a [FrameInfo], There is a mmap , you can get data there
//!
//!
//!

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

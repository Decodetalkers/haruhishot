pub mod convert;
pub mod haruhierror;
pub mod screenshot;
pub mod state;
pub mod utils;
pub use screenshot::*;
pub use state::HaruhiShotState;

fn main() {
    let state = HaruhiShotState::new(None);
}

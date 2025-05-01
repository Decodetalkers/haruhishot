pub mod convert;
pub mod haruhierror;
pub mod screenshot;
pub mod state;
pub mod utils;
pub use state::HaruhiShotState;

fn main() {
    let mut state = HaruhiShotState::new(None).unwrap();

    let output = state.outputs().first().as_ref().unwrap().output.clone();
    state.shot_output(&output);
}

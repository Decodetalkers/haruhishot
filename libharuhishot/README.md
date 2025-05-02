# libharuhishot

libharuhishot, it is used for wlr-screencopy, split it because I want to help with wayshot, but
I also learn a lot. I like my program very much, because it makes me feel alive. Wayshot is a
good program, please help them.

The lib is simple enough to use, you can take the haruhishot for example, simple usage is like

```rust
use libharuhishot::HaruhiShotState;
fn main() {
  let mut state = HaruhiShotState::init().unwrap();
  let outputs = state.outputs();
  let output = outputs[0].clone();
  let image_info = state.shot_single_output(output).unwrap();
}

```
Then you will get a [FrameInfo], There is a mmap , you can get data there

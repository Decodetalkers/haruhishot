# libharuhishot

libharuhishot, it is used for wlr-screencopy, split it because I want to help with wayshot, but
I also learn a lot. I like my program very much, because it makes me feel alive. Wayshot is a
good program, please help them.

The lib is simple enough to use, you can take the haruhishot for example, simple useage is like

```rust
use libharuhishot::HaruhiShotState;
fn main() {
  let mut state = HaruhiShotState::init().unwrap();
  let buffer = state.capture_out_frame(
       &state.display[0].clone,
       state.display_logic_size[0],
       state.display_transform[id],
       None
  ).unwrap();
}

```
Then you will get a [FrameInfo], There is a mmap , you can get data there

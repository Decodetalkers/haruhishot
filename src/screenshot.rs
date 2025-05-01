use wayland_client::protocol::wl_output::WlOutput;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::Options;

use crate::{HaruhiShotState, state::FrameInfo};

impl HaruhiShotState {
    fn shot_output(&self, output: &WlOutput) {
        let img_manager = self.output_image_manager();
        let capture_manager = self.image_copy_capture_manager();
        let qh = self.qhandle();
        let source = img_manager.create_source(output, qh, ());
        let frame_info = FrameInfo::default();
        let session =
            capture_manager.create_session(&source, Options::PaintCursors, qh, frame_info);
        let frame = session.create_frame(qh, ());
    }
}

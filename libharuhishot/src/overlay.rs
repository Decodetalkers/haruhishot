use std::collections::HashSet;

use wayland_client::{Connection, QueueHandle, delegate_noop, protocol::wl_output::WlOutput};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self, ZwlrLayerSurfaceV1,
};

use wayland_client::protocol::{
    wl_buffer::WlBuffer, wl_compositor::WlCompositor, wl_shm::WlShm, wl_shm_pool::WlShmPool,
    wl_surface::WlSurface,
};

use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};

use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

#[derive(Debug)]
pub(crate) struct LayerShellState {
    pub configured_outputs: HashSet<WlOutput>,
}

impl LayerShellState {
    pub(crate) fn new() -> Self {
        Self {
            configured_outputs: HashSet::new(),
        }
    }
}

delegate_noop!(LayerShellState: ignore WlCompositor);
delegate_noop!(LayerShellState: ignore WlShm);
delegate_noop!(LayerShellState: ignore WlShmPool);
delegate_noop!(LayerShellState: ignore WlBuffer);
delegate_noop!(LayerShellState: ignore ZwlrLayerShellV1);
delegate_noop!(LayerShellState: ignore WlSurface);
delegate_noop!(LayerShellState: ignore WpViewport);
delegate_noop!(LayerShellState: ignore WpViewporter);

impl wayland_client::Dispatch<ZwlrLayerSurfaceV1, WlOutput> for LayerShellState {
    // No need to instrument here, span from lib.rs is automatically used.
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as wayland_client::Proxy>::Event,
        data: &WlOutput,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width: _,
                height: _,
            } => {
                tracing::debug!("Acking configure");
                state.configured_outputs.insert(data.clone());

                proxy.ack_configure(serial);
                tracing::trace!("Acked configure");
            }
            zwlr_layer_surface_v1::Event::Closed => {
                tracing::debug!("Closed")
            }
            _ => {}
        }
    }
}

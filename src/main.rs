use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
    ext_image_copy_capture_manager_v1::{self, ExtImageCopyCaptureManagerV1},
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1::{
        self, ExtForeignToplevelImageCaptureSourceManagerV1,
    },
    ext_image_capture_source_v1::{self, ExtImageCaptureSourceV1},
    ext_output_image_capture_source_manager_v1::{self, ExtOutputImageCaptureSourceManagerV1},
};

use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

use wayland_client::{
    Connection, Dispatch, Proxy, delegate_noop, event_created_child,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_output::WlOutput, wl_registry, wl_shm::WlShm},
};

struct HaruhiShotState;

delegate_noop!(HaruhiShotState: ignore ExtImageCaptureSourceV1);
delegate_noop!(HaruhiShotState: ignore ExtOutputImageCaptureSourceManagerV1);
delegate_noop!(HaruhiShotState: ignore ExtForeignToplevelImageCaptureSourceManagerV1);
delegate_noop!(HaruhiShotState: ignore WlShm);

delegate_noop!(HaruhiShotState: ignore ExtImageCopyCaptureManagerV1);

impl Dispatch<wl_registry::WlRegistry, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == WlOutput::interface().name {
            } else if interface == WlShm::interface().name {
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<ExtForeignToplevelListV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        println!("{event:?}");
    }
    event_created_child!(HaruhiShotState, ExtForeignToplevelHandleV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ())
    ]);
}
impl Dispatch<ExtForeignToplevelHandleV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &ExtForeignToplevelHandleV1,
        event: <ExtForeignToplevelHandleV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let ext_foreign_toplevel_handle_v1::Event::Title { title } = event {
            println!("{title}");
        }
    }
}

struct BaseState;
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for BaseState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let mut state = HaruhiShotState;
    let (globals, _) = registry_queue_init::<BaseState>(&conn).unwrap(); // We just need the

    let mut event_queue = conn.new_event_queue::<HaruhiShotState>();

    let qh = event_queue.handle();
    let manager = globals
        .bind::<ExtImageCopyCaptureManagerV1, _, _>(&qh, 1..=1, ())
        .unwrap();
    let toplist = globals
        .bind::<ExtForeignToplevelListV1, _, _>(&qh, 1..=1, ())
        .unwrap();
    event_queue.blocking_dispatch(&mut state).unwrap();
}

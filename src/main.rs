use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
    ext_image_copy_capture_manager_v1::{self, ExtImageCopyCaptureManagerV1},
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
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
    protocol::{
        wl_buffer::WlBuffer,
        wl_output::{self, WlOutput},
        wl_registry,
        wl_shm::WlShm,
    },
};

use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1,
    zxdg_output_v1::{self, ZxdgOutputV1},
};

use std::sync::OnceLock;

pub mod utils;

use utils::*;

#[derive(Debug)]
struct WlOutputInfo {
    output: WlOutput,
    size: Size,
    logical_size: Size,
    position: Position,
    logical_position: Position,
    name: String,
    xdg_output: OnceLock<ZxdgOutputV1>,
}

impl WlOutputInfo {
    fn new(output: WlOutput) -> Self {
        Self {
            output,
            position: Position::default(),
            logical_position: Position::default(),
            size: Size::default(),
            logical_size: Size::default(),
            name: "".to_owned(),
            xdg_output: OnceLock::new(),
        }
    }
}

#[derive(Debug)]
pub struct TopLevel {
    handle: ExtForeignToplevelHandleV1,
    title: String,
}

impl TopLevel {
    fn new(handle: ExtForeignToplevelHandleV1) -> Self {
        Self {
            handle,
            title: "".to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub struct HaruhiShotState {
    toplevels: Vec<TopLevel>,
    output_infos: Vec<WlOutputInfo>,
}

impl HaruhiShotState {
    pub fn new() -> Self {
        Self::default()
    }
}

delegate_noop!(HaruhiShotState: ignore ExtImageCaptureSourceV1);
delegate_noop!(HaruhiShotState: ignore ExtOutputImageCaptureSourceManagerV1);
delegate_noop!(HaruhiShotState: ignore ExtForeignToplevelImageCaptureSourceManagerV1);
delegate_noop!(HaruhiShotState: ignore WlShm);
delegate_noop!(HaruhiShotState: ignore ZxdgOutputManagerV1);
delegate_noop!(HaruhiShotState: ignore ExtImageCopyCaptureManagerV1);
delegate_noop!(HaruhiShotState: ignore WlBuffer);

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &ExtImageCopyCaptureSessionV1,
        event: <ExtImageCopyCaptureSessionV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &ExtImageCopyCaptureFrameV1,
        event: <ExtImageCopyCaptureFrameV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == WlOutput::interface().name {
                state
                    .output_infos
                    .push(WlOutputInfo::new(proxy.bind(name, version, &qh, ())));
            } else if interface == WlShm::interface().name {
            }
        }
    }
}

impl Dispatch<ZxdgOutputV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &ZxdgOutputV1,
        event: <ZxdgOutputV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let Some(data) =
            state
                .output_infos
                .iter_mut()
                .find(|WlOutputInfo { xdg_output, .. }| {
                    xdg_output.get().expect("we need to init here") == proxy
                })
        else {
            return;
        };

        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                data.logical_position = Position { x, y }
            }
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                data.logical_size = Size { width, height };
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let Some(data) = state
            .output_infos
            .iter_mut()
            .find(|WlOutputInfo { output, .. }| output == proxy)
        else {
            return;
        };
        match event {
            wl_output::Event::Name { name } => {
                data.name = name;
            }
            _ => {}
        }
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
        if let ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } = event {
            state.toplevels.push(TopLevel::new(toplevel));
        }
    }
    event_created_child!(HaruhiShotState, ExtForeignToplevelHandleV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ())
    ]);
}
impl Dispatch<ExtForeignToplevelHandleV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        toplevel: &ExtForeignToplevelHandleV1,
        event: <ExtForeignToplevelHandleV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let ext_foreign_toplevel_handle_v1::Event::Title { title } = event else {
            return;
        };
        let Some(current_info) = state
            .toplevels
            .iter_mut()
            .find(|my_toplevel| my_toplevel.handle == *toplevel)
        else {
            return;
        };
        current_info.title = title;
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for HaruhiShotState {
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

    let mut state = HaruhiShotState::new();
    let (globals, mut event_queue) = registry_queue_init::<HaruhiShotState>(&conn).unwrap(); // We just need the

    let qh = event_queue.handle();

    let _registry = display.get_registry(&qh, ());
    event_queue.blocking_dispatch(&mut state).unwrap();
    let imageManager = globals
        .bind::<ExtImageCopyCaptureManagerV1, _, _>(&qh, 1..=1, ())
        .unwrap();

    globals
        .bind::<ExtForeignToplevelListV1, _, _>(&qh, 1..=1, ())
        .unwrap();
    let the_xdg_output_manager = globals
        .bind::<ZxdgOutputManagerV1, _, _>(&qh, 3..=3, ())
        .unwrap();

    for output in state.output_infos.iter_mut() {
        let xdg_the_output = the_xdg_output_manager.get_xdg_output(&output.output, &qh, ());
        output.xdg_output.set(xdg_the_output).unwrap();
    }

    event_queue.blocking_dispatch(&mut state).unwrap();
}

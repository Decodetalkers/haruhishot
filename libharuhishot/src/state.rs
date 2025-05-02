use wayland_client::{EventQueue, WEnum};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1, FailureReason},
    ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
};

use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop, event_created_child,
    globals::{GlobalList, GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer::WlBuffer,
        wl_output::{self, WlOutput},
        wl_registry,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
    },
};

use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1,
    zxdg_output_v1::{self, ZxdgOutputV1},
};

use std::sync::{Arc, OnceLock, RwLock};

use crate::haruhierror::HaruhiError;
use crate::utils::*;

#[derive(Debug, Default)]
pub struct HaruhiShotState {
    toplevels: Vec<TopLevel>,
    output_infos: Vec<WlOutputInfo>,
    img_copy_manager: OnceLock<ExtImageCopyCaptureManagerV1>,
    output_image_manager: OnceLock<ExtOutputImageCaptureSourceManagerV1>,
    shm: OnceLock<WlShm>,
    qh: OnceLock<QueueHandle<Self>>,
    event_queue: Option<EventQueue<Self>>,
    conn: OnceLock<Connection>,
    globals: OnceLock<GlobalList>,
}

impl HaruhiShotState {
    pub(crate) fn image_copy_capture_manager(&self) -> &ExtImageCopyCaptureManagerV1 {
        self.img_copy_manager.get().expect("Should init")
    }
    pub(crate) fn output_image_manager(&self) -> &ExtOutputImageCaptureSourceManagerV1 {
        self.output_image_manager.get().expect("Should init")
    }
    pub(crate) fn qhandle(&self) -> &QueueHandle<Self> {
        self.qh.get().expect("Should init")
    }

    pub(crate) fn take_event_queue(&mut self) -> EventQueue<Self> {
        self.event_queue.take().expect("control your self")
    }

    pub(crate) fn reset_event_queue(&mut self, event_queue: EventQueue<Self>) {
        self.event_queue = Some(event_queue);
    }

    pub(crate) fn shm(&self) -> &WlShm {
        self.shm.get().expect("Should init")
    }

    pub fn outputs(&self) -> &Vec<WlOutputInfo> {
        &self.output_infos
    }

    pub fn connection(&self) -> &Connection {
        self.conn.get().expect("should init")
    }

    pub fn globals(&self) -> &GlobalList {
        self.globals.get().expect("should init")
    }
}

pub struct HaruhiConnection<'a> {
    pub conn: &'a Connection,
    pub globals: &'a GlobalList,
}

impl HaruhiShotState {
    pub fn print_display_info(&self) {
        for WlOutputInfo {
            size: Size { width, height },
            logical_size:
                Size {
                    width: logical_width,
                    height: logical_height,
                },
            position: Position { x, y },
            name,
            description,
            scale,
            ..
        } in self.outputs()
        {
            println!("{name}, {description}");
            println!("    Size: {width},{height}");
            println!("    LogicSize: {logical_width}, {logical_height}");
            println!("    Position: {x}, {y}");
            println!("    Scale: {scale}");
        }
    }

    pub fn init() -> Result<Self, HaruhiError> {
        let conn = Connection::connect_to_env()?;

        let (globals, mut event_queue) = registry_queue_init::<HaruhiShotState>(&conn)?; // We just need the
        let display = conn.display();

        let mut state = HaruhiShotState::default();

        let qh = event_queue.handle();

        let _registry = display.get_registry(&qh, ());
        event_queue.blocking_dispatch(&mut state)?;
        let image_manager = globals.bind::<ExtImageCopyCaptureManagerV1, _, _>(&qh, 1..=1, ())?;
        let output_image_manager =
            globals.bind::<ExtOutputImageCaptureSourceManagerV1, _, _>(&qh, 1..=1, ())?;
        let shm = globals.bind::<WlShm, _, _>(&qh, 1..=2, ())?;
        globals.bind::<ExtForeignToplevelListV1, _, _>(&qh, 1..=1, ())?;
        let the_xdg_output_manager = globals.bind::<ZxdgOutputManagerV1, _, _>(&qh, 3..=3, ())?;

        for output in state.output_infos.iter_mut() {
            let xdg_the_output = the_xdg_output_manager.get_xdg_output(&output.output, &qh, ());
            output.xdg_output.set(xdg_the_output).unwrap();
        }

        event_queue.blocking_dispatch(&mut state)?;

        state.img_copy_manager.set(image_manager).unwrap();
        state
            .output_image_manager
            .set(output_image_manager)
            .unwrap();
        state.qh.set(qh).unwrap();
        state.shm.set(shm).unwrap();
        state.globals.set(globals).unwrap();
        state.conn.set(conn).unwrap();
        state.event_queue = Some(event_queue);
        Ok(state)
    }
}

delegate_noop!(HaruhiShotState: ignore ExtImageCaptureSourceV1);
delegate_noop!(HaruhiShotState: ignore ExtOutputImageCaptureSourceManagerV1);
delegate_noop!(HaruhiShotState: ignore ExtForeignToplevelImageCaptureSourceManagerV1);
delegate_noop!(HaruhiShotState: ignore WlShm);
delegate_noop!(HaruhiShotState: ignore ZxdgOutputManagerV1);
delegate_noop!(HaruhiShotState: ignore ExtImageCopyCaptureManagerV1);
delegate_noop!(HaruhiShotState: ignore WlBuffer);
delegate_noop!(HaruhiShotState: ignore WlShmPool);

#[derive(Debug, Default)]
pub(crate) struct FrameInfo {
    buffer_size: OnceLock<Size<u32>>,
    shm_format: OnceLock<WEnum<Format>>,
}

impl FrameInfo {
    pub(crate) fn size(&self) -> Size<u32> {
        self.buffer_size.get().cloned().expect("not inited")
    }

    pub(crate) fn format(&self) -> WEnum<Format> {
        self.shm_format.get().cloned().expect("Not inited")
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, Arc<RwLock<FrameInfo>>> for HaruhiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtImageCopyCaptureSessionV1,
        event: <ExtImageCopyCaptureSessionV1 as Proxy>::Event,
        data: &Arc<RwLock<FrameInfo>>,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let frame_info = data.write().unwrap();
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                frame_info
                    .buffer_size
                    .set(Size { width, height })
                    .expect("should set only once");
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat { format } => {
                frame_info
                    .shm_format
                    .set(format)
                    .expect("should set only once");
            }
            ext_image_copy_capture_session_v1::Event::Done => {}
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CaptureState {
    Failed(WEnum<FailureReason>),
    Succeeded,
    Pedding,
}

pub(crate) struct CaptureInfo {
    transform: wl_output::Transform,
    state: CaptureState,
}

impl CaptureInfo {
    pub(crate) fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            transform: wl_output::Transform::Normal,
            state: CaptureState::Pedding,
        }))
    }

    pub(crate) fn transform(&self) -> wl_output::Transform {
        self.transform
    }
    pub(crate) fn state(&self) -> CaptureState {
        self.state
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, Arc<RwLock<CaptureInfo>>> for HaruhiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtImageCopyCaptureFrameV1,
        event: <ExtImageCopyCaptureFrameV1 as Proxy>::Event,
        data: &Arc<RwLock<CaptureInfo>>,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let mut data = data.write().unwrap();
        match event {
            ext_image_copy_capture_frame_v1::Event::Ready => {
                data.state = CaptureState::Succeeded;
            }
            ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
                data.state = CaptureState::Failed(reason)
            }
            ext_image_copy_capture_frame_v1::Event::Transform {
                transform: WEnum::Value(transform),
            } => {
                data.transform = transform;
            }
            _ => {}
        }
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
                    .push(WlOutputInfo::new(proxy.bind(name, version, qh, ())));
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
            zxdg_output_v1::Event::LogicalPosition { x, y } => data.position = Position { x, y },
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                data.logical_size = Size { width, height };
            }
            zxdg_output_v1::Event::Description { description } => {
                data.description = description;
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
            wl_output::Event::Scale { factor } => {
                data.scale = factor;
            }
            wl_output::Event::Mode { width, height, .. } => {
                data.size = Size { width, height };
            }
            wl_output::Event::Geometry {
                transform: WEnum::Value(transform),
                ..
            } => {
                data.transform = transform;
            }
            _ => {}
        }
    }
}
impl Dispatch<ExtForeignToplevelListV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
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
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
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

use wayland_client::protocol::wl_output::{self, Transform, WlOutput};
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};
use wayland_client::{Proxy, WEnum};

use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1;
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::{self, ZxdgOutputV1};

// wlr
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use std::iter::zip;

use std::sync::{Arc, Mutex};

use crate::harihierror::HarihiError;
use crate::wlrcopystate::WlrCopyStateInfo;

use wayland_client::EventQueue;
// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.

pub struct HarihiShotState {
    // global information
    pub displays: Vec<WlOutput>,
    pub display_names: Vec<String>,
    pub display_description: Vec<String>,
    pub display_size: Vec<(i32, i32)>,
    pub display_position: Vec<(i32, i32)>,
    pub display_scale: Vec<i32>,
    pub display_logic_size: Vec<(i32, i32)>,
    pub display_transform: Vec<Transform>,
    pub(crate) shm: Option<WlShm>,
    pub(crate) wlr_screencopy: Option<ZwlrScreencopyManagerV1>,
    pub(crate) xdg_output_manager: Option<ZxdgOutputManagerV1>,

    // copy state
    pub(crate) wlr_copy_state_info: WlrCopyStateInfo,

    pub(crate) queue: Option<Arc<Mutex<EventQueue<Self>>>>,
}

impl HarihiShotState {
    pub fn init() -> Result<Self, HarihiError> {
        // Create a Wayland connection by connecting to the server through the
        // environment-provided configuration.
        let conn = Connection::connect_to_env()
            .map_err(|_| HarihiError::InitFailed("Error During connection".to_string()))?;

        // Retrieve the WlDisplay Wayland object from the connection. This object is
        // the starting point of any Wayland program, from which all other objects will
        // be created.
        let display = conn.display();

        // Create an event queue for our event processing
        let mut event_queue = conn.new_event_queue();
        // An get its handle to associated new objects to it
        let qh = event_queue.handle();

        // Create a wl_registry object by sending the wl_display.get_registry request
        // This method takes two arguments: a handle to the queue the newly created
        // wl_registry will be assigned to, and the user-data that should be associated
        // with this registry (here it is () as we don't need user-data).
        let _registry = display.get_registry(&qh, ());

        // At this point everything is ready, and we just need to wait to receive the events
        // from the wl_registry, our callback will print the advertized globals.
        let mut state = HarihiShotState::new();
        event_queue
            .roundtrip(&mut state)
            .map_err(|_| HarihiError::InitFailed("Error During first roundtrip".to_string()))?;
        let xdg_output_manager = state.xdg_output_manager.clone().unwrap();
        for i in 0..state.displays.len() {
            xdg_output_manager.get_xdg_output(&state.displays[i], &qh, ());
            event_queue
                .roundtrip(&mut state)
                .map_err(|_| HarihiError::InitFailed("Error During xdg_output init".to_string()))?;
        }
        state.queue = Some(Arc::new(Mutex::new(event_queue)));

        Ok(state)
    }

    pub fn get_event_queue_handle(&self) -> Result<QueueHandle<Self>, HarihiError> {
        Ok(self
            .queue
            .as_ref()
            .unwrap()
            .lock()
            .map_err(|_| HarihiError::QueueError("Cannot unlock the queue".to_string()))?
            .handle())
    }

    pub fn blockdispatch(&mut self) -> Result<(), HarihiError> {
        let queue = self.queue.clone().unwrap();
        let mut event_queue = queue
            .lock()
            .map_err(|_| HarihiError::QueueError("Cannot unlock the queue".to_string()))?;
        event_queue
            .blocking_dispatch(self)
            .map_err(|_| HarihiError::QueueError("Error during dispatch".to_string()))?;
        Ok(())
    }

    fn new() -> Self {
        HarihiShotState {
            displays: Vec::new(),
            display_names: Vec::new(),
            display_description: Vec::new(),
            display_size: Vec::new(),
            display_position: Vec::new(),
            display_scale: Vec::new(),
            display_logic_size: Vec::new(),
            display_transform: Vec::new(),
            shm: None,
            wlr_screencopy: None,
            xdg_output_manager: None,
            wlr_copy_state_info: WlrCopyStateInfo::init(),
            queue: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        if self.displays.is_empty() {
            tracing::warn!("Cannot find any displays");
            return false;
        }
        if self.wlr_screencopy.is_none() {
            tracing::warn!("Compositer doesn't support wlr_screencopy-unstable-v1");
            return false;
        }
        if self.shm.is_none() {
            tracing::warn!("Compositer is missing wl_shm");
            return false;
        }
        if self.xdg_output_manager.is_none() {
            tracing::warn!("xdg_output_manage is missing");
            return false;
        }

        true
    }

    pub fn get_whole_screens_pos_and_region(&self) -> (i32, i32, i32, i32) {
        let (mut startx, mut starty) = (0, 0);
        let (mut endx, mut endy) = (0, 0);
        for ((width, height), (x, y)) in zip(&self.display_logic_size, &self.display_position) {
            if x < &startx {
                startx = *x;
            }
            if y < &starty {
                starty = *y;
            }
            if x + width > endx {
                endx = x + width;
            }
            if y + height > endy {
                endy = y + width;
            }
        }
        (startx, starty, endx - startx, endy - starty)
    }

    pub fn get_select_id(&self, screen: String) -> Option<usize> {
        for (i, dispay_screen) in self.display_names.iter().enumerate() {
            if dispay_screen == &screen {
                return Some(i);
            }
        }
        None
    }

    pub fn get_pos_display_id(&self, pos: (i32, i32)) -> Option<usize> {
        let (pos_x, pos_y) = pos;
        for (i, ((width, height), (x, y))) in
            zip(&self.display_logic_size, &self.display_position).enumerate()
        {
            if pos_x >= *x && pos_x <= *x + *width && pos_y >= *y && pos_y <= *y + *height {
                return Some(i);
            }
        }
        None
    }

    pub fn get_pos_display_ids(&self, pos: (i32, i32), size: (i32, i32)) -> Option<Vec<usize>> {
        let (start_x, start_y) = pos;
        let (select_width, select_height) = size;
        let (end_x, end_y) = (start_x + select_width, start_y + select_height);
        let mut ids = Vec::new();
        for (i, ((width, height), (x, y))) in
            zip(&self.display_logic_size, &self.display_position).enumerate()
        {
            // at least one point in region
            let top_left_in_region =
                start_x >= *x && start_x <= *x + *width && start_y >= *y && start_y <= *y + *height;
            let bottom_left_in_region =
                start_x >= *x && start_x <= *x + *width && end_y >= *y && end_y <= *y + *height;
            let top_right_in_region =
                end_x >= *x && end_x <= *x + *width && start_y >= *y && start_y <= *y + height;
            let bottom_right_in_region =
                end_x >= *x && end_x <= *x + *width && end_y >= *y && end_y <= *y + height;

            // on line through it;
            let left_line_through =
                start_x >= *x && start_x <= *x + width && start_y <= *y && end_y >= *y + *height;
            let right_line_through =
                end_x >= *x && end_x <= *x + width && start_y <= *y && end_y >= *y + *height;
            let top_line_through =
                start_x <= *x && end_x >= *x + width && start_y >= *y && start_y <= *y + *height;
            let bottom_line_though =
                start_x <= *x && end_x >= *x + width && end_y >= *y && end_y <= *y + *height;

            // surround
            let around = !(start_x > *x
                || start_y > *y
                || end_x > *x
                || end_y < *y + *height
                || end_x < *x + *width);

            if (top_left_in_region
                || bottom_left_in_region
                || top_right_in_region
                || bottom_right_in_region)
                || (left_line_through
                    || right_line_through
                    || top_line_through
                    || bottom_line_though)
                || around
            {
                ids.push(i);
            }
        }
        if ids.is_empty() {
            None
        } else {
            Some(ids)
        }
    }

    pub fn get_real_pos(&self, (pos_x, pos_y): (i32, i32), id: usize) -> (i32, i32) {
        (
            pos_x - self.display_position[id].0,
            pos_y - self.display_position[id].1,
        )
    }

    pub fn print_display_info(&self) {
        for (
            scale,
            ((displayname, display_description), (((logic_x, logic_y), (x, y)), (pos_x, pos_y))),
        ) in zip(
            &self.display_scale,
            zip(
                zip(&self.display_names, &self.display_description),
                zip(
                    zip(&self.display_logic_size, &self.display_size),
                    &self.display_position,
                ),
            ),
        ) {
            println!("{displayname}, {display_description}");
            println!("    Size: {x},{y}");
            println!("    LogicSize: {logic_x}, {logic_y}");
            println!("    Position: {pos_x}, {pos_y}");
            println!("    Scale: {scale}");
        }
    }
}

// Implement `Dispatch<WlRegistry, ()> for out state. This provides the logic
// to be able to process events for the wl_registry interface.
//
// The second type parameter is the user-data of our implementation. It is a
// mechanism that allows you to associate a value to each particular Wayland
// object, and allow different dispatching logic depending on the type of the
// associated value.
//
// In this example, we just use () as we don't have any value to associate. See
// the `Dispatch` documentation for more details about this.
impl Dispatch<wl_registry::WlRegistry, ()> for HarihiShotState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        conn: &Connection,
        qh: &QueueHandle<HarihiShotState>,
    ) {
        // When receiving events from the wl_registry, we are only interested in the
        // `global` event, which signals a new available global.
        // When receiving this event, we just print its characteristics in this example.
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == WlOutput::interface().name {
                state
                    .displays
                    .push(registry.bind::<WlOutput, _, _>(name, version, qh, ()));
                // get dispatch info
                let mut event_queue = conn.new_event_queue();
                event_queue.roundtrip(state).unwrap();
            } else if interface == WlShm::interface().name {
                state.shm = Some(registry.bind::<WlShm, _, _>(name, version, qh, ()));
            } else if interface == ZwlrScreencopyManagerV1::interface().name {
                state.wlr_screencopy =
                    Some(registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, version, qh, ()));
            } else if interface == ZxdgOutputManagerV1::interface().name {
                state.xdg_output_manager =
                    Some(registry.bind::<ZxdgOutputManagerV1, _, _>(name, version, qh, ()));
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for HarihiShotState {
    fn event(
        state: &mut Self,
        _proxy: &WlOutput,
        event: <WlOutput as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Name { name } => {
                state.display_names.push(name);
            }
            wl_output::Event::Description { description } => {
                state.display_description.push(description);
            }
            wl_output::Event::Mode { width, height, .. } => {
                state.display_size.push((width, height));
            }
            wl_output::Event::Scale { factor } => {
                state.display_scale.push(factor);
            }
            wl_output::Event::Geometry {
                transform: WEnum::Value(transform),
                ..
            } => {
                state.display_transform.push(transform);
            }
            _ => {}
        }
    }
}
impl Dispatch<ZxdgOutputV1, ()> for HarihiShotState {
    fn event(
        state: &mut Self,
        _proxy: &ZxdgOutputV1,
        event: <ZxdgOutputV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                state.display_position.push((x, y));
            }
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                state.display_logic_size.push((width, height));
            }
            _ => {}
        }
    }
}

impl Dispatch<ZxdgOutputManagerV1, ()> for HarihiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &ZxdgOutputManagerV1,
        _event: <ZxdgOutputManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlShm, ()> for HarihiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &WlShm,
        _event: <WlShm as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for HarihiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrScreencopyManagerV1,
        _event: <ZwlrScreencopyManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

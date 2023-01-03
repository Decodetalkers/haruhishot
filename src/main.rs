use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::Proxy;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

//use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::{self, ZxdgOutputV1};

// wlr
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

// clap

use clap::{arg, Command};

// zip
use std::iter::zip;

mod filewriter;
mod wlrbackend;
// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.
struct AppData {
    pub displays: Vec<WlOutput>,
    pub display_names: Vec<String>,
    pub display_description: Vec<String>,
    pub display_size: Vec<(i32, i32)>,
    pub shm: Option<WlShm>,
    pub wlr_screencopy: Option<ZwlrScreencopyManagerV1>,
}

impl AppData {
    fn new() -> Self {
        AppData {
            displays: Vec::new(),
            display_names: Vec::new(),
            display_description: Vec::new(),
            display_size: Vec::new(),
            shm: None,
            wlr_screencopy: None,
        }
    }

    fn is_ready(&self) -> bool {
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

        true
    }

    fn get_select_id(&self, screen: String) -> Option<usize> {
        for (i, dispay_screen) in self.display_names.iter().enumerate() {
            if dispay_screen == &screen {
                return Some(i);
            }
        }
        None
    }

    fn print_display_info(&self) {
        for ((displayname, display_description), (x, y)) in zip(
            zip(&self.display_names, &self.display_description),
            &self.display_size,
        ) {
            println!(
                "{}, {}, size: ({},{}) ",
                displayname, display_description, x, y
            );
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
impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<AppData>,
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
            } else if interface == WlShm::interface().name {
                state.shm = Some(registry.bind::<WlShm, _, _>(name, version, qh, ()));
            } else if interface == ZwlrScreencopyManagerV1::interface().name {
                state.wlr_screencopy =
                    Some(registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, version, qh, ()));
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for AppData {
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
            _ => {}
        }
    }
}
//impl Dispatch<ZxdgOutputV1, ()> for AppData {
//    fn event(
//            state: &mut Self,
//            proxy: &ZxdgOutputV1,
//            event: <ZxdgOutputV1 as Proxy>::Event,
//            data: &(),
//            conn: &Connection,
//            qhandle: &QueueHandle<Self>,
//        ) {
//        println!("ss");
//    }
//}

impl Dispatch<WlShm, ()> for AppData {
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

impl Dispatch<ZwlrScreencopyManagerV1, ()> for AppData {
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



enum ClapOption {
    ShowInfo,
    ShotWithDefaultOption,
    ShotWithCoosedScreen {
        screen: String,
    },
    ShotWithSlurp {
        pos_x: i32,
        pos_y: i32,
        width: i32,
        height: i32,
    },
}

// The main function of our program
fn main() {
    tracing_subscriber::fmt::init();

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let matches = Command::new("haruhishot")
        .about("Haruhi Suzumiya has made a wlr screenshot tool")
        .version(VERSION)
        .author("Haruhi Suzumiya")
        .subcommand(
            Command::new("output")
                .long_flag("output")
                .short_flag('O')
                .arg(arg!(<Screen> ... "Screen"))
                .about("Choose Output"),
        )
        .subcommand(
            Command::new("slurp")
                .long_flag("slurp")
                .short_flag('S')
                .arg(arg!(<Slurp> ... "Pos by slurp"))
                .about("Slurp"),
        )
        .subcommand(
            Command::new("list_outputs")
                .long_flag("list_outputs")
                .short_flag('L')
                .about("list all outputs"),
        )
        .get_matches();
    match matches.subcommand() {
        Some(("output", submatchs)) => {
            let screen = submatchs
                .get_one::<String>("Screen")
                .expect("need one screen input")
                .to_string();
            take_screenshot(ClapOption::ShotWithCoosedScreen { screen });
        }
        Some(("slurp", submatchs)) => {
            let posmessage = submatchs
                .get_one::<String>("Slurp")
                .expect("Need message")
                .to_string();
            let posmessage: Vec<&str> = posmessage.trim().split(' ').collect();
            let position: Vec<&str> = posmessage[0].split(',').collect();

            let pos_x = position[0].parse::<i32>().unwrap();
            let pos_y = position[1].parse::<i32>().unwrap();

            let map: Vec<&str> = posmessage[1].split('x').collect();
            let width = map[0].parse::<i32>().unwrap();
            let height = map[1].parse::<i32>().unwrap();
            take_screenshot(ClapOption::ShotWithSlurp {
                pos_x,
                pos_y,
                width,
                height,
            });
        }
        Some(("list_outputs", _)) => take_screenshot(ClapOption::ShowInfo),
        _ => take_screenshot(ClapOption::ShotWithDefaultOption),
    }
    //take_screenshot();
}

fn take_screenshot(option: ClapOption) {
    // Create a Wayland connection by connecting to the server through the
    // environment-provided configuration.
    let conn = Connection::connect_to_env().unwrap();

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
    let mut state = AppData::new();

    // globals.
    event_queue.roundtrip(&mut state).unwrap();

    // get output info
    event_queue.roundtrip(&mut state).unwrap();
    if state.is_ready() {
        tracing::info!("All data is ready");

        //
        match option {
            ClapOption::ShotWithDefaultOption => {
                let manager = state.wlr_screencopy.unwrap();
                let shm = state.shm.unwrap();
                let bufferdata = wlrbackend::capture_output_frame(
                    &conn,
                    &state.displays[0],
                    manager,
                    &display,
                    shm,
                    None,
                );
                match bufferdata {
                    Some(data) => filewriter::write_to_file(data),
                    None => tracing::error!("Nothing get, check the log"),
                }
            }
            ClapOption::ShotWithCoosedScreen { screen } => match state.get_select_id(screen) {
                Some(id) => {
                    let manager = state.wlr_screencopy.unwrap();
                    let shm = state.shm.unwrap();
                    let bufferdata = wlrbackend::capture_output_frame(
                        &conn,
                        &state.displays[id],
                        manager,
                        &display,
                        shm,
                        None,
                    );
                    match bufferdata {
                        Some(data) => filewriter::write_to_file(data),
                        None => tracing::error!("Nothing get, check the log"),
                    }
                }
                None => {
                    tracing::error!("Cannot find screen");
                }
            },
            ClapOption::ShowInfo => {
                state.print_display_info();
            }
            ClapOption::ShotWithSlurp {
                pos_x,
                pos_y,
                width,
                height,
            } => {
                let manager = state.wlr_screencopy.unwrap();
                let shm = state.shm.unwrap();
                // TODO: need zwlr_output to get position
                let bufferdata = wlrbackend::capture_output_frame(
                    &conn,
                    &state.displays[0],
                    manager,
                    &display,
                    shm,
                    Some((pos_x, pos_y, width, height)),
                );
                match bufferdata {
                    Some(data) => filewriter::write_to_file(data),
                    None => tracing::error!("Nothing get, check the log"),
                }
            }
        }
    }
}

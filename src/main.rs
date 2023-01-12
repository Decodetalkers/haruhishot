use dialoguer::theme::ColorfulTheme;
use dialoguer::FuzzySelect;
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::Proxy;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1;
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::{self, ZxdgOutputV1};

// wlr
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

// clap

use clap::{arg, Arg, ArgAction, Command};

// zip
use std::iter::zip;

mod constenv;
mod filewriter;
#[cfg(feature = "gui")]
mod slintbackend;
mod wlrbackend;
// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.
struct AppData {
    pub displays: Vec<WlOutput>,
    pub display_names: Vec<String>,
    pub display_description: Vec<String>,
    pub display_size: Vec<(i32, i32)>,
    display_position: Vec<(i32, i32)>,
    display_scale: Vec<i32>,
    display_logic_size: Vec<(i32, i32)>,
    pub shm: Option<WlShm>,
    pub wlr_screencopy: Option<ZwlrScreencopyManagerV1>,
    pub xdg_output_manager: Option<ZxdgOutputManagerV1>,
}

impl AppData {
    fn new() -> Self {
        AppData {
            displays: Vec::new(),
            display_names: Vec::new(),
            display_description: Vec::new(),
            display_size: Vec::new(),
            display_position: Vec::new(),
            display_scale: Vec::new(),
            display_logic_size: Vec::new(),
            shm: None,
            wlr_screencopy: None,
            xdg_output_manager: None,
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
        if self.xdg_output_manager.is_none() {
            tracing::warn!("xdg_output_manage is missing");
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

    //fn get_pos_display_id(&self, pos: (i32, i32)) -> Option<usize> {
    //    let (pos_x, pos_y) = pos;
    //    for (i, ((width, height), (x, y))) in
    //        zip(&self.display_logic_size, &self.display_postion).enumerate()
    //    {
    //        if pos_x >= *x && pos_x <= *x + *width && pos_y >= *y && pos_y <= *y + *height {
    //            return Some(i);
    //        }
    //    }
    //    None
    //}

    fn get_pos_display_ids(&self, pos: (i32, i32), size: (i32, i32)) -> Option<Vec<usize>> {
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

    fn get_real_pos(&self, pos: (i32, i32), size: (i32, i32), id: usize) -> (i32, i32, i32, i32) {
        let (x, y) = pos;
        let (width, height) = size;
        let (end_x, end_y) = (x + width, y + height);
        let (right_bottom_x, right_bottom_y) = (
            self.display_position[id].0 + self.display_logic_size[id].0,
            self.display_position[id].1 + self.display_logic_size[id].1,
        );
        let pos_x = if x - self.display_position[id].0 >= 0 {
            x - self.display_position[id].0
        } else {
            0
        };
        let pos_y = if y - self.display_position[id].1 >= 0 {
            y - self.display_position[id].1
        } else {
            0
        };

        let start_x = std::cmp::max(x, self.display_position[id].0);
        let start_y = std::cmp::max(y, self.display_position[id].1);
        let pos_end_x = std::cmp::min(end_x, right_bottom_x);
        let pos_end_y = std::cmp::min(end_y, right_bottom_y);
        (pos_x, pos_y, pos_end_x - start_x, pos_end_y - start_y)
    }
    fn print_display_info(&self) {
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
            println!("{}, {},", displayname, display_description);
            println!("    Size: {},{}", x, y);
            println!("    LogicSize: {}, {}", logic_x, logic_y);
            println!("    Position: {}, {}", pos_x, pos_y);
            println!("    Scale: {}", scale);
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
        conn: &Connection,
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
            wl_output::Event::Scale { factor } => {
                state.display_scale.push(factor);
            }
            _ => {}
        }
    }
}
impl Dispatch<ZxdgOutputV1, ()> for AppData {
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

impl Dispatch<ZxdgOutputManagerV1, ()> for AppData {
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
    ShotWithFullScreen {
        usestdout: bool,
    },
    ShotWithCoosedScreen {
        screen: Option<String>,
        usestdout: bool,
    },
    #[cfg(feature = "gui")]
    ShotWithGui,
    ShotWithSlurp {
        pos_x: i32,
        pos_y: i32,
        width: i32,
        height: i32,
        usestdout: bool,
    },
}

enum SlurpParseResult {
    Finished(i32, i32, i32, i32),
    MeetError,
}

fn parseslurp(posmessage: String) -> SlurpParseResult {
    let posmessage: Vec<&str> = posmessage.trim().split(' ').collect();
    #[cfg(feature = "notify")]
    let notify_error = |message: &str| {
        use crate::constenv::{FAILED_IMAGE, TIMEOUT};
        use notify_rust::Notification;
        let _ = Notification::new()
            .summary("FileSavedFailed")
            .body(message)
            .icon(FAILED_IMAGE)
            .timeout(TIMEOUT)
            .show();
    };
    if posmessage.len() != 2 {
        tracing::error!("Error input");
        #[cfg(feature = "notify")]
        notify_error("Get error input, Maybe canceled?");
        return SlurpParseResult::MeetError;
    }
    let position: Vec<&str> = posmessage[0].split(',').collect();

    let Ok(pos_x) = position[0]
        .parse::<i32>() else {
            tracing::error!("Error parse, Cannot get pos_x");
            #[cfg(feature = "notify")]
            notify_error("Error parse, Cannot get pos_x");
            return SlurpParseResult::MeetError;
    };
    let Ok(pos_y) = position[1]
       .parse::<i32>() else {
           tracing::error!("Error parse, Cannot get pos_y");
           #[cfg(feature = "notify")]
           notify_error("Error parse, Cannot get pos_y");
           return SlurpParseResult::MeetError;
    };

    let map: Vec<&str> = posmessage[1].split('x').collect();
    if map.len() != 2 {
        eprintln!("Error input");
        return SlurpParseResult::MeetError;
    }
    let Ok(width) = map[0]
        .parse::<i32>() else {
            tracing::error!("Error parse, cannot get width");
            #[cfg(feature = "notify")]
            notify_error("Error parse, Cannot get width");
            return SlurpParseResult::MeetError;
    };
    let Ok(height) = map[1]
        .parse::<i32>() else {
            tracing::error!("Error parse, cannot get height");
            #[cfg(feature = "notify")]
            notify_error("Error parse, Cannot get height");
            return SlurpParseResult::MeetError;
    };
    SlurpParseResult::Finished(pos_x, pos_y, width, height)
}

// The main function of our program
fn main() {
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let command = Command::new("haruhishot")
        .about("One day Haruhi Suzumiya made a wlr screenshot tool")
        .version(VERSION)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Haruhi Suzumiya")
        .subcommand(
            Command::new("output")
                .long_flag("output")
                .short_flag('O')
                .arg(arg!(<Screen> ... "Screen").required(false))
                .arg(
                    Arg::new("stdout")
                        .long("stdout")
                        .action(ArgAction::SetTrue)
                        .help("to stdout"),
                )
                .about("Choose Output"),
        )
        .subcommand(
            Command::new("slurp")
                .long_flag("slurp")
                .short_flag('S')
                .arg(arg!(<Slurp> ... "Pos by slurp"))
                .arg(
                    Arg::new("stdout")
                        .long("stdout")
                        .action(ArgAction::SetTrue)
                        .help("to stdout"),
                )
                .about("Slurp"),
        )
        .subcommand(
            Command::new("global")
                .long_flag("global")
                .short_flag('G')
                .arg(
                    Arg::new("stdout")
                        .long("stdout")
                        .action(ArgAction::SetTrue)
                        .help("to stdout"),
                )
                .about("TakeScreenshot about whole screen"),
        )
        .subcommand(
            Command::new("list_outputs")
                .long_flag("list_outputs")
                .short_flag('L')
                .about("list all outputs"),
        );
    #[cfg(feature = "gui")]
    let command = command.subcommand(
        Command::new("gui")
            .long_flag("gui")
            .about("open gui selection"),
    );
    let matches = command.get_matches();
    match matches.subcommand() {
        Some(("output", submatchs)) => {
            let usestdout = submatchs.get_flag("stdout");
            if !usestdout {
                tracing_subscriber::fmt::init();
            }
            let screen = submatchs
                .get_one::<String>("Screen")
                .map(|screen| screen.to_string());

            take_screenshot(ClapOption::ShotWithCoosedScreen { screen, usestdout });
        }
        Some(("slurp", submatchs)) => {
            let posmessage = submatchs
                .get_one::<String>("Slurp")
                .expect("Need message")
                .to_string();

            let usestdout = submatchs.get_flag("stdout");
            if !usestdout {
                tracing_subscriber::fmt::init();
            }
            let SlurpParseResult::Finished(pos_x, pos_y, width, height) = parseslurp(posmessage) else {
                return;
            };
            take_screenshot(ClapOption::ShotWithSlurp {
                pos_x,
                pos_y,
                width,
                height,
                usestdout,
            });
        }
        Some(("list_outputs", _)) => take_screenshot(ClapOption::ShowInfo),
        Some(("global", submatchs)) => {
            let usestdout = submatchs.get_flag("stdout");
            if !usestdout {
                tracing_subscriber::fmt::init();
            }
            take_screenshot(ClapOption::ShotWithFullScreen { usestdout });
        }
        #[cfg(feature = "gui")]
        Some(("gui", _)) => {
            tracing_subscriber::fmt::init();
            take_screenshot(ClapOption::ShotWithGui);
            //slintbackend::selectgui();
        }
        _ => unimplemented!(),
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

    if state.is_ready() {
        tracing::info!("All data is ready");

        let shootglobal = |usestdout: bool, state: &AppData| {
            let manager = state.wlr_screencopy.as_ref().unwrap();
            let shm = state.shm.as_ref().unwrap();
            let mut bufferdatas = Vec::new();
            for (index, wldisplay) in state.displays.iter().enumerate() {
                let Some(bufferdata) = wlrbackend::capture_output_frame(
                        &conn,
                        wldisplay,
                        manager,
                        &display,
                        shm.clone(),
                        None,
                    ) else {
                        if usestdout {
                            tracing_subscriber::fmt().init();
                        }
                        tracing::error!("Cannot get frame from screen: {} ",  state.display_names[index]);
                        #[cfg(feature = "notify")]
                        {
                            use crate::constenv::{FAILED_IMAGE, TIMEOUT};
                            use notify_rust::Notification;
                            let _ = Notification::new()
                                .summary("FileSavedFailed")
                                .body(&format!("Cannot get frame from screen: {}", state.display_names[index]))
                                .icon(FAILED_IMAGE)
                                .timeout(TIMEOUT)
                                .show();
                        }
                        return;
                    };
                bufferdatas.push(bufferdata);
            }
            filewriter::write_to_file_mutisource(bufferdatas, usestdout);
        };
        let shootchoosedscreen = |usestdout: bool, id: usize, state: &AppData| {
            let manager = state.wlr_screencopy.as_ref().unwrap();
            let shm = state.shm.clone().unwrap();
            let bufferdata = wlrbackend::capture_output_frame(
                &conn,
                &state.displays[id],
                manager,
                &display,
                shm,
                None,
            );
            match bufferdata {
                Some(data) => filewriter::write_to_file(data, usestdout),
                None => tracing::error!("Nothing get, check the log"),
            }
        };

        let shotwithslurp = |usestdout: bool,
                             state: &AppData,
                             ids: Vec<usize>,
                             posinformation: (i32, i32, i32, i32)| {
            let (pos_x, pos_y, width, height) = posinformation;
            let manager = state.wlr_screencopy.as_ref().unwrap();
            let shm = state.shm.clone().unwrap();
            let mut bufferdatas = Vec::new();
            for id in ids {
                let (pos_x, pos_y, width, height) =
                    state.get_real_pos((pos_x, pos_y), (width, height), id);
                let Some(bufferdata) = wlrbackend::capture_output_frame(
                    &conn,
                    &state.displays[id],
                    manager,
                    &display,
                    shm.clone(),
                    Some((pos_x, pos_y, width,height)),
                ) else {
                    if usestdout {
                        tracing_subscriber::fmt().init();
                    }
                    tracing::error!("Cannot get frame from screen: {} ",  state.display_names[id]);
                    #[cfg(feature = "notify")]
                    {
                        use crate::constenv::{FAILED_IMAGE, TIMEOUT};
                        use notify_rust::Notification;
                        let _ = Notification::new()
                            .summary("FileSavedFailed")
                            .body(&format!("Cannot get frame from screen: {}", state.display_names[id]))
                            .icon(FAILED_IMAGE)
                            .timeout(TIMEOUT)
                            .show();
                    }
                    return;
                };
                bufferdatas.push(bufferdata);
            }
            filewriter::write_to_file_mutisource(bufferdatas, usestdout);
        };
        //
        match option {
            ClapOption::ShotWithFullScreen { usestdout } => {
                shootglobal(usestdout, &state);
            }
            ClapOption::ShotWithCoosedScreen { screen, usestdout } => {
                let screen = match screen {
                    Some(screen) => screen,
                    None => {
                        let names = &state.display_names;
                        let Ok(selection) = FuzzySelect::with_theme(&ColorfulTheme::default())
                            .with_prompt("Choose Screen")
                            .default(0)
                            .items(&names[..])
                            .interact()
                        else {
                            if usestdout {
                                tracing_subscriber::fmt().init();
                            }
                            #[cfg(feature = "notify")]
                            {
                                use crate::constenv::{FAILED_IMAGE, TIMEOUT};
                                use notify_rust::Notification;
                                let _ = Notification::new()
                                    .summary("FileSavedFailed")
                                    .body("Unknow Screen")
                                    .icon(FAILED_IMAGE)
                                    .timeout(TIMEOUT)
                                    .show();
                            }
                            tracing::error!("You have not choose screen");
                            return;
                        };
                        names[selection].clone()
                    }
                };
                match state.get_select_id(screen) {
                    Some(id) => {
                        shootchoosedscreen(usestdout, id, &state);
                    }
                    None => {
                        #[cfg(feature = "notify")]
                        {
                            use crate::constenv::{FAILED_IMAGE, TIMEOUT};
                            use notify_rust::Notification;
                            let _ = Notification::new()
                                .summary("FileSavedFailed")
                                .body("Unknow Screen")
                                .icon(FAILED_IMAGE)
                                .timeout(TIMEOUT)
                                .show();
                        }
                        if usestdout {
                            tracing_subscriber::fmt().init();
                        }
                        tracing::error!("Cannot find screen");
                    }
                }
            }
            ClapOption::ShowInfo => {
                let xdg_output_manager = state.xdg_output_manager.clone().unwrap();
                for i in 0..state.displays.len() {
                    xdg_output_manager.get_xdg_output(&state.displays[i], &qh, ());
                    event_queue.roundtrip(&mut state).unwrap();
                }
                state.print_display_info();
            }
            #[cfg(feature = "gui")]
            ClapOption::ShotWithGui => {
                let xdg_output_manager = state.xdg_output_manager.clone().unwrap();
                for i in 0..state.displays.len() {
                    xdg_output_manager.get_xdg_output(&state.displays[i], &qh, ());
                    event_queue.roundtrip(&mut state).unwrap();
                }
                match slintbackend::selectgui(
                    state.display_names.clone(),
                    state.display_description.clone(),
                ) {
                    slintbackend::SlintSelection::GlobalScreen => {
                        shootglobal(false, &state);
                    }
                    slintbackend::SlintSelection::Selection(index) => {
                        shootchoosedscreen(false, index as usize, &state);
                    }
                    slintbackend::SlintSelection::Slurp => {
                        let Ok(output) = std::process::Command::new("slurp")
                            .arg("-d")
                            .output() else {
                                tracing::error!("Maybe Slurp Missing?");
                                #[cfg(feature = "notify")]
                                {
                                    use crate::constenv::{FAILED_IMAGE, TIMEOUT};
                                    use notify_rust::Notification;
                                    let _ = Notification::new()
                                        .summary("FileSavedFailed")
                                        .body("Maybe Slurp Missing?")
                                        .icon(FAILED_IMAGE)
                                        .timeout(TIMEOUT)
                                        .show();
                                }
                                return;
                        };
                        let message = output.stdout;
                        let posmessage = String::from_utf8_lossy(&message).to_string();
                        let SlurpParseResult::Finished(pos_x, pos_y , width , height ) = parseslurp(posmessage) else {
                            return;
                        };
                        match state.get_pos_display_ids((pos_x, pos_y), (width, height)) {
                            Some(ids) => {
                                shotwithslurp(false, &state, ids, (pos_x, pos_y, width, height));
                            }
                            None => {
                                tracing::error!("Pos is over the screen");
                                #[cfg(feature = "notify")]
                                {
                                    use crate::constenv::{FAILED_IMAGE, TIMEOUT};
                                    use notify_rust::Notification;
                                    let _ = Notification::new()
                                        .summary("FileSavedFailed")
                                        .body("Pos is over the screen")
                                        .icon(FAILED_IMAGE)
                                        .timeout(TIMEOUT)
                                        .show();
                                }
                            }
                        }
                    }
                    slintbackend::SlintSelection::Canceled => {
                        #[cfg(feature = "notify")]
                        {
                            use crate::constenv::{FAILED_IMAGE, TIMEOUT};
                            use notify_rust::Notification;
                            let _ = Notification::new()
                                .summary("Canceled")
                                .body("Canceld to Save File")
                                .icon(FAILED_IMAGE)
                                .timeout(TIMEOUT)
                                .show();
                        }
                    }
                };
            }
            ClapOption::ShotWithSlurp {
                pos_x,
                pos_y,
                width,
                height,
                usestdout,
            } => {
                let xdg_output_manager = state.xdg_output_manager.clone().unwrap();
                for i in 0..state.displays.len() {
                    xdg_output_manager.get_xdg_output(&state.displays[i], &qh, ());
                    event_queue.roundtrip(&mut state).unwrap();
                }
                match state.get_pos_display_ids((pos_x, pos_y), (width, height)) {
                    Some(ids) => {
                        shotwithslurp(usestdout, &state, ids, (pos_x, pos_y, width, height));
                    }
                    None => {
                        tracing::error!("Pos is over the screen");
                        #[cfg(feature = "notify")]
                        {
                            use crate::constenv::{FAILED_IMAGE, TIMEOUT};
                            use notify_rust::Notification;
                            let _ = Notification::new()
                                .summary("FileSavedFailed")
                                .body("Pos is over the screen")
                                .icon(FAILED_IMAGE)
                                .timeout(TIMEOUT)
                                .show();
                        }
                    }
                }
            }
        }
    } else {
        #[cfg(feature = "notify")]
        {
            use crate::constenv::{FAILED_IMAGE, TIMEOUT};
            use notify_rust::Notification;
            let _ = Notification::new()
                .summary("FileSavedFailed")
                .body("Cannot get Data")
                .icon(FAILED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        }
        tracing::error!("You have not choose screen");
    }
}

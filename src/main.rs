use dialoguer::theme::ColorfulTheme;
use dialoguer::FuzzySelect;
use wayland_client::protocol::wl_output;

use clap::{arg, Arg, ArgAction, Command};

mod constenv;
mod filewriter;
#[cfg(feature = "gui")]
mod slintbackend;
#[cfg(feature = "sway")]
mod swayloop;

use libharuhishot::HarihiShotState;
// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.

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
    ShotWithColor {
        pos_x: i32,
        pos_y: i32,
    },
    #[cfg(feature = "sway")]
    ShotWindow,
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
            Command::new("color")
                .long_flag("color")
                .short_flag('C')
                .arg(arg!(<Point> ... "Pos by slurp"))
                .about("Get Color of a point"),
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
    #[cfg(feature = "sway")]
    let command = command.subcommand(
        Command::new("window")
            .long_flag("window")
            .about("select window"),
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
        Some(("color", submatchs)) => {
            let posmessage = submatchs
                .get_one::<String>("Point")
                .expect("Need message")
                .to_string();
            let SlurpParseResult::Finished(pos_x, pos_y, _, _) = parseslurp(posmessage) else {
                return;
            };
            tracing_subscriber::fmt().init();
            take_screenshot(ClapOption::ShotWithColor { pos_x, pos_y })
        }
        #[cfg(feature = "gui")]
        Some(("gui", _)) => {
            tracing_subscriber::fmt::init();
            take_screenshot(ClapOption::ShotWithGui);
            //slintbackend::selectgui();
        }
        #[cfg(feature = "sway")]
        Some(("window", _)) => {
            tracing_subscriber::fmt::init();
            take_screenshot(ClapOption::ShotWindow);
        }
        _ => unimplemented!(),
    }
    //take_screenshot();
}

fn take_screenshot(option: ClapOption) {
    let mut state = HarihiShotState::init().unwrap();

    if state.is_ready() {
        tracing::info!("All data is ready");

        let shoot_choosed_screen = |usestdout: bool, id: usize, state: &mut HarihiShotState| {
            let bufferdata = state.capture_output_frame(
                &state.displays[id].clone(),
                state.display_logic_size[id],
                state.display_transform[id],
                None,
            );
            match bufferdata {
                Ok(Some(data)) => filewriter::write_to_file(data, usestdout),
                Ok(None) => tracing::error!("Nothing get, check the log"),
                Err(e) => eprintln!("Error: {e}"),
            }
        };

        let shot_with_regions =
            |usestdout: bool,
             state: &mut HarihiShotState,
             ids: Vec<usize>,
             posinformation: (i32, i32, i32, i32)| {
                let (pos_x, pos_y, width, height) = posinformation;
                let mut bufferdatas = Vec::new();
                for id in ids {
                    let (pos_x, pos_y) = state.get_real_pos((pos_x, pos_y), id);
                    // INFO: sometime I get 0
                    if width == 0 || height == 0 {
                        continue;
                    }
                    let Ok(Some(bufferdata)) = state.capture_output_frame(
                        &state.displays[id].clone(),
                        (width, height),
                        state.display_transform[id],
                        Some((pos_x, pos_y, width, height)),
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
                let region = state.get_whole_screens_pos_and_region();
                let allscreens: Vec<usize> = (0..state.displays.len()).collect();
                shot_with_regions(usestdout, &mut state, allscreens, region);
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
                        shoot_choosed_screen(usestdout, id, &mut state);
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
            ClapOption::ShotWithColor { pos_x, pos_y } => {
                if let Some(id) = state.get_pos_display_id((pos_x, pos_y)) {
                    let (pos_x, pos_y) = state.get_real_pos((pos_x, pos_y), id);
                    if let Ok(Some(bufferdata)) = state.capture_output_frame(
                        &state.displays[id].clone(),
                        (1, 1),
                        wl_output::Transform::Normal,
                        Some((pos_x, pos_y, 1, 1)),
                    ) {
                        filewriter::get_color(bufferdata);
                    }
                }
            }
            ClapOption::ShowInfo => {
                state.print_display_info();
            }
            #[cfg(feature = "gui")]
            ClapOption::ShotWithGui => {
                match slintbackend::selectgui(
                    state.display_names.clone(),
                    state.display_description.clone(),
                ) {
                    slintbackend::SlintSelection::GlobalScreen => {
                        let region = state.get_whole_screens_pos_and_region();
                        let allscreens: Vec<usize> = (0..state.displays.len()).collect();
                        shot_with_regions(false, &mut state, allscreens, region);
                    }
                    slintbackend::SlintSelection::Selection(index) => {
                        shoot_choosed_screen(false, index as usize, &mut state);
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
                                shot_with_regions(
                                    false,
                                    &mut state,
                                    ids,
                                    (pos_x, pos_y, width, height),
                                );
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
            } => match state.get_pos_display_ids((pos_x, pos_y), (width, height)) {
                Some(ids) => {
                    shot_with_regions(usestdout, &mut state, ids, (pos_x, pos_y, width, height));
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
            },
            #[cfg(feature = "sway")]
            ClapOption::ShotWindow => {
                swayloop::get_window();
                swayloop::swaylayer();
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    if let Ok(can_exit) = swayloop::CAN_EXIT.lock() {
                        if let swayloop::SwayWindowSelect::Finish = *can_exit {
                            break;
                        }
                    }
                }

                if let Ok(window) = swayloop::FINAL_WINDOW.lock() {
                    let (pos_x, pos_y, width, height) = *window;
                    println!("{pos_x},{pos_y},{width}, {height}");
                    match state.get_pos_display_ids((pos_x, pos_y), (width, height)) {
                        Some(ids) => {
                            shot_with_regions(
                                false,
                                &mut state,
                                ids,
                                (pos_x, pos_y, width, height),
                            );
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

use std::rc::Rc;

use slint::VecModel;
use std::iter::zip;
slint::include_modules!();

use std::sync::mpsc;

pub enum SlintSelection {
    GlobalScreen,
    Slurp,
    Canceled,
    Selection(i32),
}

fn init_slots(ui: &AppWindow, sender: mpsc::Sender<SlintSelection>) {
    let global = SelectSlots::get(ui);
    let sender_slurp = sender.clone();
    global.on_useSlurp(move || {
        let _ = sender_slurp.send(SlintSelection::Slurp);
        let _ = slint::quit_event_loop();
    });
    let sender_global = sender.clone();
    global.on_useGlobal(move || {
        let _ = sender_global.send(SlintSelection::GlobalScreen);
        let _ = slint::quit_event_loop();
    });

    global.on_selectScreen(move |index| {
        let _ = sender.send(SlintSelection::Selection(index));
        let _ = slint::quit_event_loop();
    });
}

pub fn selectgui(screen: Vec<String>, screeninfo: Vec<String>) -> SlintSelection {
    let ui = AppWindow::new();
    ui.set_infos(
        Rc::new(VecModel::from(
            zip(screen, screeninfo)
                .into_iter()
                .map(|(screen, info)| ScreenInfo {
                    name: screen.into(),
                    description: info.into(),
                })
                .collect::<Vec<ScreenInfo>>(),
        ))
        .into(),
    );
    let (sender, receiver) = mpsc::channel();
    init_slots(&ui, sender);
    ui.run();
    if let Ok(message) = receiver.recv_timeout(std::time::Duration::from_nanos(300)) {
        message
    } else {
        SlintSelection::Canceled
    }
}

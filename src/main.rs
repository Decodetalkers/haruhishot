//use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1 as zwlcopy;
mod backend;
mod convert;
mod outputs;
use backend::EncodingFormat;
//mod outputs;
use image::DynamicImage;
use outputs::*;
use std::env;
use std::error::Error;
use std::io::Cursor;
use std::sync::mpsc::{channel, Sender};
use wayland_client::{protocol::wl_output::WlOutput, Display};

slint::include_modules!();

fn popupwindow(sender: Sender<(i32, EncodingFormat)>) {
    let ui = AppWindow::new();
    let globalselects = SelectPopUpSlots::get(&ui);
    //globalselects.on_currentselect(move |index| {
    //    let _ = sender.send(index);
    //    let _ = slint::quit_event_loop();
    //});
    globalselects.on_currentselect(move |index, picture| {
        match picture {
            1 => {
                let _ = sender.send((index, EncodingFormat::Jpg));
            }
            2 => {
                let _ = sender.send((index, EncodingFormat::Ppm));
            }
            _ => {
                let _ = sender.send((index, EncodingFormat::Png));
            }
        };
        let _ = slint::quit_event_loop();
    });
    ui.set_shotpage(0);
    ui.run();
}

fn imagewindow(image: DynamicImage) {
    let ui = AppWindow::new();
    let globalimage = globalImage::get(&ui);
    globalimage.set_image(slint::Image::from_rgba8(
        slint::SharedPixelBuffer::clone_from_slice(image.as_bytes(), image.width(), image.height()),
    ));
    ui.set_shotpage(1);
    ui.run();
}

fn main() -> Result<(), Box<dyn Error>> {
    env::set_var("RUST_LOG", "greateshot=info");
    env_logger::init();
    log::info!("start");
    let display = Display::connect_to_env()?;
    //let outputs = get_all_outputs(display.clone());
    let output: WlOutput = get_all_outputs(display.clone())
        .first()
        .unwrap()
        .wl_output
        .clone();

    let (stdout_tx, stdout_rs) = channel();
    popupwindow(stdout_tx);

    let (cursor, encodingformat) = stdout_rs
        .recv_timeout(std::time::Duration::from_nanos(300))
        .unwrap_or_else(|_| (0, EncodingFormat::Png));

    let frame_copy = backend::capture_output_frame(display, cursor, output, None)?;
    let mut buff = Cursor::new(Vec::new());

    backend::write_to_file(&mut buff, encodingformat, frame_copy)?;
    match image::load_from_memory_with_format(buff.get_ref(), image::ImageFormat::Png) {
        Ok(image) => imagewindow(image),
        Err(e) => log::error!("err: {e}"),
    };

    Ok(())
}

mod clapargs;

use clap::Parser;
use dialoguer::FuzzySelect;
use dialoguer::theme::ColorfulTheme;
use image::codecs::png::PngEncoder;
use image::{GenericImageView, ImageEncoder, ImageError, Rgba};
pub use libharuhishot::HaruhiShotState;
use libharuhishot::reexport::Transform;
use libharuhishot::{
    CaptureOption, ClipImageViewInfoArea, ClipRegion, ImageInfo, Position, Region, Size,
};

use std::io::{BufWriter, Write, stdout};
use std::{env, fs, path::PathBuf};

use std::sync::LazyLock;

use clapargs::HaruhiCli;

const TMP: &str = "/tmp";

pub const SUCCEED_IMAGE: &str = "haruhi_succeeded";
pub const FAILED_IMAGE: &str = "haruhi_failed";
pub const TIMEOUT: i32 = 10000;

pub static SAVEPATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let Ok(home) = env::var("HOME") else {
        return PathBuf::from(TMP);
    };
    let targetpath = PathBuf::from(home).join("Pictures").join("haruhishot");
    if !targetpath.exists() && fs::create_dir_all(&targetpath).is_err() {
        return PathBuf::from(TMP);
    }
    targetpath
});

fn random_file_path() -> PathBuf {
    let file_name = format!(
        "{}-haruhui.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    SAVEPATH.join(file_name)
}

#[derive(Debug, thiserror::Error)]
enum HaruhiImageWriteError {
    #[error("Image Error")]
    ImageError(#[from] ImageError),
    #[error("file created failed")]
    FileCreatedFailed(#[from] std::io::Error),
    #[error("FuzzySelect Failed")]
    FuzzySelectFailed(#[from] dialoguer::Error),
    #[error("Output not exist")]
    OutputNotExist,
    #[error("Wayland shot error")]
    WaylandError(#[from] libharuhishot::Error),
}

#[derive(Debug, Clone)]
enum HaruhiShotResult {
    StdoutSucceeded,
    SaveToFile(PathBuf),
    ColorSucceeded,
}

trait ToCaptureOption {
    fn to_capture_option(self) -> CaptureOption;
}

impl ToCaptureOption for bool {
    fn to_capture_option(self) -> CaptureOption {
        if self {
            CaptureOption::PaintCursors
        } else {
            CaptureOption::None
        }
    }
}

fn capture_toplevel(
    state: &mut HaruhiShotState,
    use_stdout: bool,
    pointer: bool,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let toplevels = state.toplevels();
    let names: Vec<String> = toplevels.iter().map(|info| info.id_and_title()).collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose Application")
        .default(0)
        .items(&names)
        .interact()?;

    let toplevel = toplevels[selection].clone();
    let image_info = state.capture_toplevel(pointer.to_capture_option(), toplevel)?;

    write_to_image(image_info, use_stdout)
}

fn capture_output(
    state: &mut HaruhiShotState,
    output: Option<String>,
    use_stdout: bool,
    pointer: bool,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let outputs = state.outputs();
    let names: Vec<&str> = outputs.iter().map(|info| info.name()).collect();

    let selection = match output {
        Some(name) => names
            .iter()
            .position(|tname| *tname == name)
            .ok_or(HaruhiImageWriteError::OutputNotExist)?,
        None => FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose Screen")
            .default(0)
            .items(&names)
            .interact()?,
    };

    let output = outputs[selection].clone();
    let image_info = state.capture_single_output(pointer.to_capture_option(), output)?;

    write_to_image(image_info, use_stdout)
}

fn capture_area(
    state: &mut HaruhiShotState,
    use_stdout: bool,
    pointer: bool,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let views = state.capture_area(pointer.to_capture_option(), |w_conn: &HaruhiShotState| {
        let info = libwaysip::WaySip::new()
            .with_connection(w_conn.connection().clone())
            .with_selection_type(libwaysip::SelectionType::Area)
            .get()
            .map_err(|e| libharuhishot::Error::CaptureFailed(e.to_string()))?
            .ok_or(libharuhishot::Error::CaptureFailed(
                "Failed to capture the area".to_string(),
            ))?;
        waysip_to_region(info.size(), info.left_top_point())
    })?;
    // Calculate the total canvas size
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    let mut start_x = i32::MAX;
    let mut start_y = i32::MAX;
    for view in &views.areas {
        let Position { x, y } = view.region.display_position_real();
        let Size { width, height } = view.region.display_logical_size();

        start_x = start_x.min(x);
        start_y = start_y.min(y);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + width);
        max_y = max_y.max(y + height);
    }
    let total_width = (max_x - min_x) as u32;
    let total_height = (max_y - min_y) as u32;

    let mut combined_image = image::RgbaImage::new(total_width, total_height);
    for ClipImageViewInfoArea {
        info:
            ImageInfo {
                data,
                width: img_width,
                height: img_height,
                transform,
                ..
            },
        region,
    } in views.areas
    {
        // Load the captured image
        let img = image::ImageBuffer::from_raw(img_width, img_height, data).ok_or(
            HaruhiImageWriteError::ImageError(ImageError::Parameter(
                image::error::ParameterError::from_kind(
                    image::error::ParameterErrorKind::DimensionMismatch,
                ),
            )),
        )?;

        let img = match transform {
            Transform::Normal => img,
            Transform::_90 => image::imageops::rotate90(&img),
            Transform::_180 => image::imageops::rotate180(&img),
            Transform::_270 => image::imageops::rotate270(&img),
            Transform::Flipped => image::imageops::flip_vertical(&img),
            Transform::Flipped90 => {
                image::imageops::flip_vertical(&image::imageops::rotate90(&img))
            }
            Transform::Flipped180 => {
                image::imageops::flip_vertical(&image::imageops::rotate180(&img))
            }
            Transform::Flipped270 => {
                image::imageops::flip_vertical(&image::imageops::rotate270(&img))
            }
            _ => unreachable!(),
        };
        let Size { width, height } = region.display_logical_size();
        let img = image::imageops::resize(
            &img,
            width as u32,
            height as u32,
            image::imageops::FilterType::Gaussian,
        );
        // we use the real position to calculate the position
        let Position { x, y } = region.display_position_real();
        // Calculate the position in he combined image
        let offset_x = (x - min_x) as u32;
        let offset_y = (y - min_y) as u32;

        // Copy the output image to the combined image
        for (x, y, pixel) in img.enumerate_pixels() {
            let target_x = offset_x + x;
            let target_y = offset_y + y;
            if target_x < total_width && target_y < total_height {
                combined_image.put_pixel(target_x, target_y, *pixel);
            }
        }
    }
    let clip_region = views.region;
    let image = combined_image
        .view(
            (clip_region.position.x - start_x) as u32,
            (clip_region.position.y - start_y) as u32,
            clip_region.size.width as u32,
            clip_region.size.height as u32,
        )
        .to_image();

    if use_stdout {
        let mut buff = std::io::Cursor::new(Vec::new());
        image.write_to(&mut buff, image::ImageFormat::Png)?;
        let content = buff.get_ref();
        let stdout = stdout();
        let mut writer = BufWriter::new(stdout.lock());
        writer.write_all(content)?;
        Ok(HaruhiShotResult::StdoutSucceeded)
    } else {
        let file = random_file_path();
        image.save(&file)?;
        Ok(HaruhiShotResult::SaveToFile(file))
    }
}
fn get_color(state: &mut HaruhiShotState) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let mut views = state.capture_area(CaptureOption::None, |w_conn: &HaruhiShotState| {
        let info = libwaysip::WaySip::new()
            .with_connection(w_conn.connection().clone())
            .with_selection_type(libwaysip::SelectionType::Point)
            .get()
            .map_err(|e| libharuhishot::Error::CaptureFailed(e.to_string()))?
            .ok_or(libharuhishot::Error::CaptureFailed(
                "Failed to capture the area".to_string(),
            ))?;
        waysip_to_region(info.size(), info.left_top_point())
    })?;
    let ClipImageViewInfoArea {
        info:
            ImageInfo {
                data,
                width: img_width,
                height: img_height,
                transform,
                ..
            },
        region:
            ClipRegion {
                relative_region_real:
                    Region {
                        position: Position { x, y },
                        size: Size { width, height },
                    },
                ..
            },
    } = views.areas.remove(0);
    let image: image::ImageBuffer<Rgba<u8>, Vec<u8>> =
        image::ImageBuffer::from_raw(img_width, img_height, data).unwrap();
    let img = match transform {
        Transform::Normal => image,
        Transform::_90 => image::imageops::rotate90(&image),
        Transform::_180 => image::imageops::rotate180(&image),
        Transform::_270 => image::imageops::rotate270(&image),
        Transform::Flipped => image::imageops::flip_vertical(&image),
        Transform::Flipped90 => image::imageops::flip_vertical(&image::imageops::rotate90(&image)),
        Transform::Flipped180 => {
            image::imageops::flip_vertical(&image::imageops::rotate180(&image))
        }
        Transform::Flipped270 => {
            image::imageops::flip_vertical(&image::imageops::rotate270(&image))
        }
        _ => unreachable!(),
    };

    let clipimage = img.view(x as u32, y as u32, width as u32, height as u32);
    let pixel = clipimage.get_pixel(0, 0);
    println!(
        "RGB: R:{}, G:{}, B:{}, A:{}",
        pixel.0[0], pixel.0[1], pixel.0[2], pixel[3]
    );
    println!(
        "16hex: #{:02x}{:02x}{:02x}{:02x}",
        pixel.0[0], pixel.0[1], pixel.0[2], pixel[3]
    );
    Ok(HaruhiShotResult::ColorSucceeded)
}

fn notify_result(shot_result: Result<HaruhiShotResult, HaruhiImageWriteError>) {
    use notify_rust::Notification;
    match shot_result {
        Ok(HaruhiShotResult::StdoutSucceeded) => {
            let _ = Notification::new()
                .summary("Screenshot Succeed")
                .body("Screenshot Succeed")
                .icon(SUCCEED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        }
        Ok(HaruhiShotResult::SaveToFile(file)) => {
            let file_name = file.to_string_lossy().to_string();
            let _ = Notification::new()
                .summary("File Saved SUcceed")
                .body(format!("File Saved to {file:?}").as_str())
                .icon(&file_name)
                .timeout(TIMEOUT)
                .show();
        }
        Ok(HaruhiShotResult::ColorSucceeded) => {}
        Err(e) => {
            let _ = Notification::new()
                .summary("File Saved Failed")
                .body(&e.to_string())
                .icon(FAILED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        }
    }
}

fn capture_fullscreen(
    state: &mut HaruhiShotState,
    use_stdout: bool,
    pointer: bool,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let outputs = state.outputs().clone();
    if outputs.is_empty() {
        return Err(HaruhiImageWriteError::OutputNotExist);
    }

    // Calculate the total canvas size
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for output in &outputs {
        let position = output.position();
        let size = output.logical_size();
        let x = position.x;
        let y = position.y;
        let width = size.width;
        let height = size.height;

        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + width);
        max_y = max_y.max(y + height);
    }

    let total_width = (max_x - min_x) as u32;
    let total_height = (max_y - min_y) as u32;

    // Create a new image with the total size
    let mut combined_image = image::RgbaImage::new(total_width, total_height);

    // Capture each output and copy to the combined image
    for output in outputs {
        let Size { width, height } = output.logical_size();
        let image_info =
            state.capture_single_output(pointer.to_capture_option(), output.clone())?;
        let image =
            image::ImageBuffer::from_raw(image_info.width, image_info.height, image_info.data)
                .ok_or(HaruhiImageWriteError::ImageError(ImageError::Parameter(
                    image::error::ParameterError::from_kind(
                        image::error::ParameterErrorKind::DimensionMismatch,
                    ),
                )))?;
        let image = match image_info.transform {
            Transform::Normal => image,
            Transform::_90 => image::imageops::rotate90(&image),
            Transform::_180 => image::imageops::rotate180(&image),
            Transform::_270 => image::imageops::rotate270(&image),
            Transform::Flipped => image::imageops::flip_vertical(&image),
            Transform::Flipped90 => {
                image::imageops::flip_vertical(&image::imageops::rotate90(&image))
            }
            Transform::Flipped180 => {
                image::imageops::flip_vertical(&image::imageops::rotate180(&image))
            }
            Transform::Flipped270 => {
                image::imageops::flip_vertical(&image::imageops::rotate270(&image))
            }
            _ => unreachable!(),
        };
        // Load the captured image
        let img = image::imageops::resize(
            &image,
            width as u32,
            height as u32,
            image::imageops::FilterType::Gaussian,
        );

        let rgba_img: image::RgbaImage = img;

        // Calculate the position in the combined image
        let position = output.position();
        let offset_x = (position.x - min_x) as u32;
        let offset_y = (position.y - min_y) as u32;

        // Copy the output image to the combined image
        for (x, y, pixel) in rgba_img.enumerate_pixels() {
            let target_x = offset_x + x;
            let target_y = offset_y + y;
            if target_x < total_width && target_y < total_height {
                combined_image.put_pixel(target_x, target_y, *pixel);
            }
        }
    }

    let combined_image_info = ImageInfo {
        data: combined_image.into_raw(),
        width: total_width,
        height: total_height,
        color_type: image::ColorType::Rgba8,
        transform: libharuhishot::reexport::Transform::Normal,
    };

    write_to_image(combined_image_info, use_stdout)
}

pub fn waysip_to_region(
    size: libwaysip::Size,
    point: libwaysip::Position,
) -> Result<Region, libharuhishot::Error> {
    let size: Size = Size {
        width: size.width,
        height: size.height,
    };
    let position: Position = Position {
        x: point.x,
        y: point.y,
    };

    Ok(Region { position, size })
}

fn capture_all_outputs(
    state: &mut HaruhiShotState,
    use_stdout: bool,
    pointer: bool,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let image_info = state.capture_all_outputs(pointer.to_capture_option())?;
    write_to_image(image_info, use_stdout)
}

fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let args = HaruhiCli::parse();
    let mut state =
        HaruhiShotState::new().expect("Your wm needs to support Image Copy Capture protocol");

    match args {
        HaruhiCli::ListOutputs => {
            state.print_displays_info();
        }
        HaruhiCli::Application {
            stdout,
            cursor: pointer,
        } => notify_result(capture_toplevel(&mut state, stdout, pointer)),
        HaruhiCli::Output {
            output,
            stdout,
            cursor: pointer,
        } => notify_result(capture_output(&mut state, output, stdout, pointer)),
        HaruhiCli::Fullscreen {
            stdout,
            cursor: pointer,
        } => notify_result(capture_fullscreen(&mut state, stdout, pointer)),
        HaruhiCli::Slurp {
            stdout,
            cursor: pointer,
        } => {
            notify_result(capture_area(&mut state, stdout, pointer));
        }
        HaruhiCli::Color => {
            notify_result(get_color(&mut state));
        }
        HaruhiCli::AllOutputs {
            stdout,
            cursor: pointer,
        } => notify_result(capture_all_outputs(&mut state, stdout, pointer)),
    }
}

fn write_to_image(
    image_info: ImageInfo,
    use_stdout: bool,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    if use_stdout {
        write_to_stdout(image_info)
    } else {
        write_to_file(image_info)
    }
}

fn write_to_stdout(
    ImageInfo {
        data,
        width,
        height,
        color_type,
        ..
    }: ImageInfo,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let stdout = stdout();
    let mut writer = BufWriter::new(stdout.lock());
    PngEncoder::new(&mut writer).write_image(&data, width, height, color_type.into())?;
    Ok(HaruhiShotResult::StdoutSucceeded)
}

fn write_to_file(
    ImageInfo {
        data,
        width,
        height,
        color_type,
        ..
    }: ImageInfo,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let file = random_file_path();
    let mut writer =
        std::fs::File::create(&file).map_err(HaruhiImageWriteError::FileCreatedFailed)?;

    PngEncoder::new(&mut writer).write_image(&data, width, height, color_type.into())?;
    Ok(HaruhiShotResult::SaveToFile(file))
}

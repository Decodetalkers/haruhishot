mod clapargs;

use clap::Parser;
use dialoguer::FuzzySelect;
use dialoguer::theme::ColorfulTheme;
use image::codecs::png::PngEncoder;
use image::{GenericImageView, ImageEncoder, ImageError};
pub use libharuhishot::HaruhiShotState;
use libharuhishot::{CaptureOption, ImageInfo, ImageViewInfo, Position, Region, Size};

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

fn shot_output(
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

fn shot_area(
    state: &mut HaruhiShotState,
    use_stdout: bool,
    pointer: bool,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let ImageViewInfo {
        info:
            ImageInfo {
                data,
                width: img_width,
                height: img_height,
                color_type,
            },
        region:
            Region {
                position: Position { x, y },
                size: Size { width, height },
            },
    } = state.capture_area(pointer.to_capture_option(), |w_conn: &HaruhiShotState| {
        let info = libwaysip::get_area(
            Some(libwaysip::WaysipConnection {
                connection: w_conn.connection(),
                globals: w_conn.globals(),
            }),
            libwaysip::SelectionType::Area,
        )
        .map_err(|e| libharuhishot::Error::CaptureFailed(e.to_string()))?
        .ok_or(libharuhishot::Error::CaptureFailed(
            "Failed to capture the area".to_string(),
        ))?;
        waysip_to_region(info.size(), info.left_top_point())
    })?;

    let mut buff = std::io::Cursor::new(Vec::new());
    PngEncoder::new(&mut buff).write_image(&data, img_width, img_height, color_type.into())?;
    let img = image::load_from_memory_with_format(buff.get_ref(), image::ImageFormat::Png).unwrap();
    let clipimage = img.view(x as u32, y as u32, width as u32, height as u32);
    if use_stdout {
        let mut buff = std::io::Cursor::new(Vec::new());
        clipimage
            .to_image()
            .write_to(&mut buff, image::ImageFormat::Png)?;
        let content = buff.get_ref();
        let stdout = stdout();
        let mut writer = BufWriter::new(stdout.lock());
        writer.write_all(content)?;
        Ok(HaruhiShotResult::StdoutSucceeded)
    } else {
        let file = random_file_path();
        clipimage.to_image().save(&file)?;
        Ok(HaruhiShotResult::SaveToFile(file))
    }
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

pub fn waysip_to_region(
    size: libwaysip::Size,
    point: libwaysip::Point,
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

fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let args = HaruhiCli::parse();
    let mut state =
        HaruhiShotState::init().expect("Your wm needs to support Image Copy Capture protocol");

    match args {
        HaruhiCli::ListOutputs => {
            state.print_displays_info();
        }
        HaruhiCli::Output {
            output,
            stdout,
            cursor: pointer,
        } => notify_result(shot_output(&mut state, output, stdout, pointer)),
        HaruhiCli::Slurp {
            stdout,
            cursor: pointer,
        } => {
            notify_result(shot_area(&mut state, stdout, pointer));
        }
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
    }: ImageInfo,
) -> Result<HaruhiShotResult, HaruhiImageWriteError> {
    let file = random_file_path();
    let mut writer =
        std::fs::File::create(&file).map_err(HaruhiImageWriteError::FileCreatedFailed)?;

    PngEncoder::new(&mut writer).write_image(&data, width, height, color_type.into())?;
    Ok(HaruhiShotResult::SaveToFile(file))
}

mod clapargs;

use clap::Parser;
use dialoguer::FuzzySelect;
use dialoguer::theme::ColorfulTheme;
use image::codecs::png::PngEncoder;
use image::{ImageEncoder, ImageError};
pub use libharuhishot::HaruhiShotState;
use libharuhishot::ImageInfo;

use std::io::{BufWriter, stdout};
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

fn shot_output(
    state: &mut HaruhiShotState,
    output: Option<String>,
    use_stdout: bool,
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
            .interact()
            .map_err(HaruhiImageWriteError::FuzzySelectFailed)?,
    };

    let output = outputs[selection].clone();
    let image_info = state
        .shot_single_output(&output)
        .map_err(HaruhiImageWriteError::WaylandError)?;

    write_to_image(image_info, use_stdout)
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

fn main() {
    let args = HaruhiCli::parse();
    let mut state = HaruhiShotState::new(None).unwrap();

    match args {
        HaruhiCli::ListOutputs => {
            state.print_display_info();
        }
        HaruhiCli::Output { output, stdout } => {
            notify_result(shot_output(&mut state, output, stdout))
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
    PngEncoder::new(&mut writer)
        .write_image(&data, width, height, color_type.into())
        .map_err(HaruhiImageWriteError::ImageError)?;
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

    PngEncoder::new(&mut writer)
        .write_image(&data, width, height, color_type.into())
        .map_err(HaruhiImageWriteError::ImageError)?;
    Ok(HaruhiShotResult::SaveToFile(file))
}

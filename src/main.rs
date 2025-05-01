use image::ImageEncoder;
use image::codecs::png::PngEncoder;
pub use libharuhishot::HaruhiShotState;
use libharuhishot::ImageInfo;

use std::{env, fs, path::PathBuf};

use std::sync::LazyLock;

const TMP: &str = "/tmp";

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

fn main() {
    let mut state = HaruhiShotState::new(None).unwrap();

    let output = state.outputs().first().as_ref().unwrap().output().clone();
    let ImageInfo {
        data,
        width,
        height,
        color_type,
    } = state.shot_output(&output).unwrap();

    let file = random_file_path();
    let mut writer = std::fs::File::create(&file).unwrap();

    PngEncoder::new(&mut writer)
        .write_image(data.as_slice(), width, height, color_type.into())
        .unwrap();
}

use std::{env, fs, path::PathBuf};

use once_cell::sync::Lazy;

const TMP: &str = "/tmp";

pub static SAVEPATH: Lazy<PathBuf> = Lazy::new(|| {
    let Ok(home) = env::var("HOME") else {
        return PathBuf::from(TMP);
    };
    let targetpath = PathBuf::from(home).join("Pictures").join("haruhishot");
    if !targetpath.exists() && fs::create_dir_all(&targetpath).is_err() {
        return PathBuf::from(TMP);
    }
    targetpath
});

#[cfg(feature = "notify")]
pub const SUCCESSED_IMAGE: &str = "haruhi_successed";
#[cfg(feature = "notify")]
pub const FAILED_IMAGE: &str = "haruhi_failed";
#[cfg(feature = "notify")]
pub const TIMEOUT: i32 = 10000;

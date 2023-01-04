use std::{env, fs, path::PathBuf};

use once_cell::sync::Lazy;

const TMP: &str = "/tmp";

pub static SAVEPATH: Lazy<PathBuf> = Lazy::new(|| {
    let Ok(home) = env::var("HOME") else {
        return PathBuf::from(TMP);
    };
    let targetpath = PathBuf::from(home).join("Pictures").join("haruhishot");
    if !targetpath.exists() {
        if fs::create_dir_all(&targetpath).is_err() {
            return PathBuf::from(TMP);
        }
    }
    targetpath
});

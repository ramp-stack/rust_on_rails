use platform_dirs::AppDirs;
use std::fs;

pub fn get_application_support_dir() -> Option<String> {
    let app_dirs = match AppDirs::new(Some("orange"), true) {
        Some(dirs) => dirs,
        None => return None,
    };

    let data_dir = app_dirs.data_dir;

    if let Err(_) = fs::create_dir_all(&data_dir) {
        return None;
    }

    data_dir.to_str().map(String::from)
}
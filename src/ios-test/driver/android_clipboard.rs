#[cfg(target_os = "android")]
use android_clipboard::get_text;

#[cfg(target_os = "android")]
pub fn get_clipboard_string() -> Option<String> {
    let clipboard_value = get_text();

    match clipboard_value {
        Ok(string_value) => Some(string_value),
        Err(e) => None,
    }
}
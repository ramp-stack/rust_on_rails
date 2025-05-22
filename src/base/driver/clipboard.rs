#[cfg(not(any(target_os = "ios", target_os = "android")))]
use cli_clipboard;
#[cfg(target_os = "ios")]
use objc2_ui_kit::UIPasteboard;

pub struct Clipboard;

impl Clipboard {
    #[cfg(target_os = "ios")]
    pub fn get() -> String {
        unsafe {
            let pasteboard = UIPasteboard::generalPasteboard();
            pasteboard.string().map(|s| s.to_string()).unwrap_or_default() 
        }
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    pub fn get() -> String {
        cli_clipboard::get_contents().unwrap_or_default()
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    pub fn set(text: String) {
        cli_clipboard::set_contents(text).unwrap();
    }
}

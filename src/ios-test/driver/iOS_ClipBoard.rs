#[cfg(any(target_os = "ios"))]
use objc2_ui_kit::UIPasteboard;

#[cfg(any(target_os = "ios"))]
pub unsafe fn get_clipboard_string() -> Option<String> {
    let pasteboard = UIPasteboard::generalPasteboard();
    pasteboard.string().map(|s| s.to_string())
}
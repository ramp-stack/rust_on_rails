#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::rc::autoreleasepool;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_app_kit::{NSPasteboard, NSPasteboardType};

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub fn get_clipboard_string() -> Option<String> {
    autoreleasepool(|_| unsafe {
        let pasteboard = NSPasteboard::generalPasteboard();
        let available_types = pasteboard.types()?;
        let string_type = NSPasteboardType::string();
        if available_types.containsObject(&string_type) {
            if let Some(copied_string) = pasteboard.stringForType(&string_type) {
                return Some(copied_string.to_string());
            }
        }

        None
    })
}
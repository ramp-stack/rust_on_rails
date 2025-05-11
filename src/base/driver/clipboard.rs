#[cfg(not(any(target_os = "ios", target_os = "android")))]
use cli_clipboard;

#[cfg(target_os = "ios")]
extern "C" {
    fn get_clipboard_string() -> *const std::os::raw::c_char;
}

pub struct Clipboard;

impl Clipboard {
    pub fn get() -> String {
        #[cfg(target_os = "ios")]
        unsafe {
            let ptr = get_clipboard_string();
            if ptr.is_null() {
                return String::new();
            }

            let cstr = std::ffi::CStr::from_ptr(ptr);;
            return cstr.to_string_lossy().into_owned()
        }
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        return cli_clipboard::get_contents().unwrap_or_default();
    }

    pub fn set(text: String) {
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        cli_clipboard::set_contents(text).unwrap();
    }
}
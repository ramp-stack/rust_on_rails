#[cfg(target_os = "ios")]
extern "C" {
    fn get_clipboard_string() -> *const std::os::raw::c_char;
}

pub struct Clipboard;

impl Clipboard {
    #[cfg(target_os = "ios")]
    pub fn get() -> String {
        unsafe {
            let ptr = get_clipboard_string();
            if ptr.is_null() {
                return String::new();
            }

            let cstr = std::ffi::CStr::from_ptr(ptr);
            cstr.to_string_lossy().into_owned()
        }
    }
}

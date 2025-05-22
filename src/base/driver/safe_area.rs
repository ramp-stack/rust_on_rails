#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::rc::Retained;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::MainThreadMarker;
#[cfg(any(target_os = "ios"))]
use objc2_ui_kit::UIApplication;
use std::sync::Once;

static mut INSETS: [f64; 4] = [0.0; 4];
static INIT: Once = Once::new();

pub struct SafeAreaInsets;

impl SafeAreaInsets {
    #[cfg(target_os = "ios")]
    pub fn get(&self) -> [f64; 4] {
        unsafe {
            INIT.call_once(|| {
                let mtm = MainThreadMarker::new().expect("must be on the main thread");
                let window: Retained<UIApplication> = UIApplication::sharedApplication(mtm);
    
                if let Some(key_window) = window.keyWindow() {
                    let insets = key_window.safeAreaInsets();
    
                    INSETS[0] = insets.top as f64;
    
                    INSETS[1] = insets.bottom as f64;
    
                    INSETS[2] = insets.left as f64;
    
                    INSETS[3] = insets.right as f64;
                }
            });
    
            INSETS
        } 
    }

    #[cfg(not(target_os = "ios"))]
    pub fn get(&self) -> [f64; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

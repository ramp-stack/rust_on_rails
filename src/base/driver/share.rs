#[cfg(target_os = "ios")]
use objc2::{class, msg_send};
#[cfg(target_os = "ios")]
use objc2_foundation::{NSArray, NSString, NSObject};
#[cfg(target_os = "ios")]
use objc2::rc::autoreleasepool;

pub struct Share;

impl Share {
    #[cfg(target_os = "ios")]
    pub fn share(text: &str) {
        autoreleasepool(|_| {
            let ns_string = NSString::from_str(text);
            let items = NSArray::from_slice(&[&*ns_string]);
    
            let cls = class!(UIActivityViewController);
            let activity_controller: *mut NSObject = unsafe { msg_send![cls, alloc] };
    
            let activity_controller: *mut NSObject = unsafe {
                msg_send![activity_controller, initWithActivityItems:&*items, applicationActivities: std::ptr::null_mut::<NSArray<NSObject>>()]
            };
    
            let ui_app = class!(UIApplication);
            let shared_app: *mut NSObject = unsafe { msg_send![ui_app, sharedApplication] };
            let key_window: *mut NSObject = unsafe { msg_send![shared_app, keyWindow] };
            let root_vc: *mut NSObject = unsafe { msg_send![key_window, rootViewController] };
    
            let _: () = unsafe {
                msg_send![
                    root_vc,
                    presentViewController: activity_controller,
                    animated: true,
                    completion: std::ptr::null_mut::<objc2::runtime::NSObject>()
                ]
            };
        });
    }
    
    #[cfg(target_os = "macos")]
    pub fn share(_text: &str) {}
    #[cfg(target_os = "linux")]
    pub fn share(_text: &str) {}
    #[cfg(target_os = "android")]
    pub fn share(_text: &str) {}
}

#[cfg(target_os = "ios")]
use objc2::{class, msg_send, sel};
#[cfg(target_os = "ios")]
use objc2_foundation::{NSArray, NSString, NSObject, NSData};
#[cfg(target_os = "ios")]
use objc2::rc::autoreleasepool;
#[cfg(target_os = "ios")]
use objc2::rc::Id;
#[cfg(target_os = "ios")]
use objc2::rc::AutoreleasePool;

pub struct Share;

impl Share {
    #[cfg(target_os = "ios")]
    
    pub fn share(text: &str) {
        autoreleasepool(|_| {
            let ns_string = NSString::from_str(text);
            let ns_string_obj: Id<NSObject> = unsafe { std::mem::transmute(ns_string) };

            let image_name = NSString::from_str("ShareIcon");
            let image: *mut NSObject = unsafe { msg_send![class!(UIImage), imageNamed: &*image_name] };
            let image_obj: Id<NSObject> = unsafe {
                Id::retain(image.cast()).expect("Image 'ShareIcon' not found in asset catalog")
            };

            let items = NSArray::from_slice(&[&*ns_string_obj, &*image_obj]);

            let activity_controller: *mut NSObject = unsafe {
                let cls = class!(UIActivityViewController);
                let alloc: *mut NSObject = msg_send![cls, alloc];
                msg_send![
                    alloc,
                    initWithActivityItems: &*items,
                    applicationActivities: std::ptr::null_mut::<NSArray<NSObject>>()
                ]
            };

            let app: *mut NSObject = unsafe { msg_send![class!(UIApplication), sharedApplication] };
            let window: *mut NSObject = unsafe { msg_send![app, keyWindow] };
            let root_vc: *mut NSObject = unsafe { msg_send![window, rootViewController] };

            let _: () = unsafe {
                msg_send![
                    root_vc,
                    presentViewController: activity_controller,
                    animated: true,
                    completion: std::ptr::null_mut::<NSObject>()
                ]
            };
        });
    }

    
    #[cfg(target_os = "macos")]
    pub fn share(text: &str) {}
    #[cfg(target_os = "linux")]
    pub fn share(text: &str) {}
    #[cfg(target_os = "android")]
    pub fn share(text: &str) {}
}

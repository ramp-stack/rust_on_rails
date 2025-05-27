use objc2::declare_class;
use objc2::rc::autoreleasepool;
use objc2::runtime::{NSObject, Object};
use objc2::{class, msg_send, sel, ClassType};
use objc2_foundation::{NSArray, NSString, NSURL};
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;

static mut IMAGE_PATH_SENDER: Option<Mutex<Sender<String>>> = None;

const PICKER_DELEGATE_RUST_NAME: &str = "PickerDelegateRust";

fn ns_string(s: &str) -> &NSString {
    NSString::from_str(s).as_ref()
}

declare_class!(
    #[name("PickerDelegateRust")]
    pub struct PickerDelegate;

    unsafe impl ClassType for PickerDelegate {
        type Super = NSObject;
    }

    unsafe impl PickerDelegate {
        #[method(sel = "picker:didFinishPicking:")]
        fn picker_did_finish_picking(
            &self,
            picker: &Object,
            results: &NSArray<Object>,
        ) {
            // Dismiss picker UI on main thread
            unsafe {
                let _: () = msg_send![picker, dismissViewControllerAnimated: true completion: std::ptr::null_mut::<()>()];
            }

            if results.count() == 0 {
                return;
            }

            let first_result = unsafe { results.object_at(0) };

            let item_provider: *mut Object = unsafe { msg_send![first_result, itemProvider] };

            // Get the sender from global state
            let sender = unsafe {
                IMAGE_PATH_SENDER
                    .as_ref()
                    .and_then(|mutex| mutex.lock().ok())
                    .map(|s| s.clone())
            };

            if sender.is_none() {
                eprintln!("No sender available to send image path");
                return;
            }

            let sender = sender.unwrap();

            use objc2::block::{Block, ConcreteBlock};

            let block = ConcreteBlock::new(move |url: *mut Object, error: *mut Object| {
                if !url.is_null() {
                    let ns_url = unsafe { NSURL::from_ptr(url) };
                    if let Some(path) = ns_url.path().to_str() {
                        let _ = sender.send(path.to_owned());
                    }
                } else {
                    eprintln!("Error loading image URL from PHPicker: {:?}", error);
                }
            });
            let block = Block::copy(&block);

            unsafe {
                let _: () = msg_send![item_provider,
                    loadFileRepresentationForTypeIdentifier: ns_string("public.image")
                    completionHandler: &*block
                ];
            }
        }
    }
);


pub struct CameraRoll;

impl CameraRoll {
    #[cfg(target_os = "ios")]
    pub fn get() -> Option<String> {
        let (sender, receiver) = channel();
        unsafe {
            IMAGE_PATH_SENDER = Some(Mutex::new(sender));
        }

        autoreleasepool(|_| {
            let config_cls = class!(PHPickerConfiguration);
            let config: *mut Object = unsafe { msg_send![config_cls, new] };

            let picker_cls = class!(PHPickerViewController);
            let picker: *mut Object = unsafe { msg_send![picker_cls, alloc] };
            let picker: *mut Object = unsafe {
                msg_send![picker, initWithConfiguration: config]
            };

            let delegate_cls = PickerDelegate::class();
            let delegate: *mut Object = unsafe { msg_send![delegate_cls, new] };

            unsafe {
                let _: () = msg_send![picker, setDelegate: delegate];
            }

            let ui_app = class!(UIApplication);
            let shared_app: *mut Object = unsafe { msg_send![ui_app, sharedApplication] };
            let key_window: *mut Object = unsafe { msg_send![shared_app, keyWindow] };
            let root_vc: *mut Object = unsafe { msg_send![key_window, rootViewController] };

            unsafe {
                let _: () = msg_send![
                    root_vc,
                    presentViewController: picker
                    animated: true
                    completion: std::ptr::null_mut::<()>()
                ];
            }
        });

        // Wait to receive path from delegate callback (in real app use async)
        receiver.recv().ok()
    }

    #[cfg(not(target_os = "ios"))]
    pub fn get() -> Option<String> {
        None
    }
}
use objc2_foundation::{NSString, NSAutoreleasePool};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2::rc::Retained;

#[derive(Debug)]
pub struct CloudStorage;

impl CloudStorage {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn save(key: &str, value: &str) {
        unsafe {
            let _pool = NSAutoreleasePool::new();

            let store: *mut AnyObject = msg_send![class!(NSUbiquitousKeyValueStore), defaultStore];
            let ns_key: Retained<NSString> = NSString::from_str(key);
            let ns_value: Retained<NSString> = NSString::from_str(value);
            let _: () = msg_send![store, setString: &*ns_value, forKey: &*ns_key];
            let _: bool = msg_send![store, synchronize];
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn get(key: &str) -> Option<String> {
        unsafe {
            let _pool = NSAutoreleasePool::new();

            let store: *mut AnyObject = msg_send![class!(NSUbiquitousKeyValueStore), defaultStore];
            let ns_key: Retained<NSString> = NSString::from_str(key);
            let ns_value: *mut NSString = msg_send![store, stringForKey: &*ns_key];
            if ns_value.is_null() {
                None
            } else {
                Some((*ns_value).to_string())
            }
        }
    }
}

impl Default for CloudStorage {
    fn default() -> Self {
        CloudStorage
    }
}

// let cloud = CloudStorage::default();

// cloud.save("greeting", "Hello iCloud!");

// if let Some(value) = cloud.get("greeting") {
//     println!("Got value: {}", value);
// } else {
//     println!("No value found.");
// }
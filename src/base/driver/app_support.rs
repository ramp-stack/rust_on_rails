use std::fs;
use objc2_foundation::{
    NSArray, 
    NSString, 
    NSAutoreleasePool, 
    NSFileManager, 
    NSURL, 
    NSObject,
    NSError,
    NSDictionary,
    NSSearchPathDirectory,
    NSSearchPathDomainMask
};
use objc2::rc::{Id, Retained};
use objc2::runtime::{Bool, Class};
use objc2::{msg_send, sel, class};
use objc2::ClassType;
use std::path::{PathBuf, Path};
use std::ffi::CStr;

pub struct ApplicationSupport;

impl ApplicationSupport {
    #[cfg(target_os = "ios")]
    pub fn get() -> Option<PathBuf> {
        unsafe {
            let _pool = NSAutoreleasePool::new();

            let file_manager = NSFileManager::defaultManager();

            let url: Result<Retained<NSURL>, Retained<objc2_foundation::NSError>> =
                file_manager.URLForDirectory_inDomain_appropriateForURL_create_error(
                    NSSearchPathDirectory::ApplicationSupportDirectory,
                    NSSearchPathDomainMask::UserDomainMask,
                    None,
                    true,
                );

            if let Some(mut url) = url.ok() {
                // let bundle: *mut NSObject = msg_send![Class::get(cstr!("NSBundle")).unwrap(), mainBundle];
                let nsbundle = Class::get(CStr::from_bytes_with_nul_unchecked(b"NSBundle\0")).unwrap();
                let bundle: *mut NSObject = msg_send![nsbundle, mainBundle];
                let identifier: *mut NSString = msg_send![bundle, bundleIdentifier];

                let identifier = if !identifier.is_null() {
                    Retained::new(identifier).unwrap()
                } else {
                    println!("No bundle identifier — using fallback.");
                    NSString::from_str("org.ramp.orange")
                };

                let subpath: Id<NSURL> = msg_send![&*url, URLByAppendingPathComponent: Retained::<NSString>::as_ptr(&identifier)];
                url = subpath;

                let _: Bool = msg_send![&*file_manager,
                    createDirectoryAtURL: &*url,
                    withIntermediateDirectories: true,
                    attributes: std::ptr::null::<objc2_foundation::NSDictionary>(),
                    error: std::ptr::null_mut::<*mut objc2_foundation::NSError>()
                ];

                let path: *mut NSString = msg_send![&*url, path];
                if !path.is_null() {
                    let str_path = (*path).to_string();
                    return Some(PathBuf::from(str_path));
                }
            }

            None
        }
    }


    #[cfg(target_os = "macos")]
    pub fn get() -> Option<PathBuf> {
        unsafe {
            let _pool = NSAutoreleasePool::new();

            let file_manager = NSFileManager::defaultManager();

            let url: Result<Retained<NSURL>, Retained<NSError>> = file_manager.URLForDirectory_inDomain_appropriateForURL_create_error(
                NSSearchPathDirectory::ApplicationSupportDirectory,
                NSSearchPathDomainMask::UserDomainMask,
                None,
                true
            );

            println!("URL: {:?}", url);

            if let Some(mut url) = url.ok() {
                let bundle: *mut NSObject = msg_send![class!(NSBundle), mainBundle];
                println!("Bundle {:?}", bundle);
                let identifier: *mut NSString = msg_send![bundle, bundleIdentifier];
                println!("Identfier {:?}", identifier);


                let identifier = if !identifier.is_null() {
                    Id::retain(identifier).unwrap()
                } else {
                    println!("Running outside .app bundle — using fallback identifier");
                    NSString::from_str("org.ramp.orange")
                };

                let subpath: Id<NSURL> = msg_send![&*url, URLByAppendingPathComponent: Retained::<NSString>::as_ptr(&identifier)];
                url = subpath;

                let _: Bool = msg_send![&*file_manager,
                    createDirectoryAtURL: &*url,
                    withIntermediateDirectories: true,
                    attributes: std::ptr::null::<NSDictionary>(),
                    error: std::ptr::null_mut::<*mut NSError>()
                ];

                let path: *mut NSString = msg_send![&*url, path];
                if !path.is_null() {
                    let str_path = (*path).to_string();
                    return Some(PathBuf::from(str_path));
                }
            }

            None
        }
    }
}

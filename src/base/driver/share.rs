#[cfg(target_os = "ios")]
use objc2::{class, msg_send};
#[cfg(target_os = "ios")]
use objc2_foundation::{NSArray, NSString, NSObject};
#[cfg(target_os = "ios")]
use objc2::rc::autoreleasepool;

#[cfg(target_os = "android")]
use jni::{JNIEnv, JavaVM};
#[cfg(target_os = "android")]
use jni::objects::{JClass, JObject, JString, JValue, GlobalRef};
#[cfg(target_os = "android")]
use jni::sys::{jobject, jint, JNI_VERSION_1_6};
#[cfg(target_os = "android")]
use std::sync::{Mutex, OnceLock};
#[cfg(target_os = "android")]
use std::ffi::CString;

#[cfg(target_os = "android")]
static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();
#[cfg(target_os = "android")]
static CURRENT_ACTIVITY: Mutex<Option<GlobalRef>> = Mutex::new(None);

pub struct Share {
    #[cfg(target_os = "android")]
    java_vm: Option<&'static JavaVM>,
}

impl Share {
    #[cfg(target_os = "android")]
    pub fn new() -> Self {
        Self {
            java_vm: JAVA_VM.get(),
        }
    }

    #[cfg(not(target_os = "android"))]
    pub fn new() -> Self {
        Self {}
    }

    #[cfg(target_os = "ios")]
    pub fn share(text: &str) {
        autoreleasepool(|_| {
            let ns_string = NSString::from_str(text);
            let items = NSArray::from_slice(&[&*ns_string]);

            let cls = class!(UIActivityViewController);
            let activity_controller: *mut NSObject = unsafe { msg_send![cls, alloc] };

            let activity_controller: *mut NSObject = unsafe {
                msg_send![activity_controller, initWithActivityItems:&*items applicationActivities: std::ptr::null_mut::<NSArray<NSObject>>()]
            };

            let ui_app = class!(UIApplication);
            let shared_app: *mut NSObject = unsafe { msg_send![ui_app, sharedApplication] };
            let key_window: *mut NSObject = unsafe { msg_send![shared_app, keyWindow] };
            let root_vc: *mut NSObject = unsafe { msg_send![key_window, rootViewController] };

            let _: () = unsafe {
                msg_send![
                    root_vc,
                    presentViewController:activity_controller
                    animated:true
                    completion: std::ptr::null_mut::<objc2::runtime::Object>()
                ]
            };
        });
    }

    #[cfg(target_os = "android")]
    pub fn share(&self, text: &str) {
        if let Some(vm) = self.java_vm {
            if let Ok(mut env) = vm.attach_current_thread() {
                if let Err(e) = self.share_with_jni(&mut env, text) {
                    eprintln!("Failed to share on Android: {}", e);
                }
            }
        }
    }

    #[cfg(target_os = "android")]
    fn share_with_jni(&self, env: &mut JNIEnv, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create Intent for sharing
        let intent_class = env.find_class("android/content/Intent")?;
        let intent = env.new_object(intent_class, "()V", &[])?;

        // Set action to ACTION_SEND
        let action_send = env.new_string("android.intent.action.SEND")?;
        env.call_method(
            &intent,
            "setAction",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&action_send)],
        )?;

        // Set type to text/plain
        let mime_type = env.new_string("text/plain")?;
        env.call_method(
            &intent,
            "setType",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&mime_type)],
        )?;

        // Add the text to share
        let extra_text = env.new_string("android.intent.extra.TEXT")?;
        let share_text = env.new_string(text)?;
        env.call_method(
            &intent,
            "putExtra",
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&extra_text), JValue::Object(&share_text)],
        )?;

        // Create chooser
        let chooser_title = env.new_string("Share via")?;
        let intent_class_static = env.find_class("android/content/Intent")?;
        let chooser = env.call_static_method(
            intent_class_static,
            "createChooser",
            "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;",
            &[JValue::Object(&intent), JValue::Object(&chooser_title)],
        )?;

        // Get current activity and start the chooser
        let activity = Self::get_current_activity(env)?;
        let chooser_intent = chooser.l()?;
        env.call_method(
            &activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&chooser_intent)],
        )?;

        Ok(())
    }

    #[cfg(target_os = "android")]
    fn get_current_activity<'a>(env: &'a JNIEnv<'a>) -> Result<JObject<'a>, Box<dyn std::error::Error>> {
        // Get the stored current activity reference
        let activity_guard = CURRENT_ACTIVITY.lock().unwrap();
        if let Some(global_ref) = &*activity_guard {
            // Convert GlobalRef to local reference
            let local_ref = env.new_local_ref(global_ref)?;
            Ok(local_ref)
        } else {
            Err("No current activity available".into())
        }
    }

    #[cfg(target_os = "macos")]
    pub fn share(text: &str) {}

    #[cfg(target_os = "linux")]
    pub fn share(text: &str) {}
}

// Helper functions for Android activity management
#[cfg(target_os = "android")]
pub fn set_current_activity(env: &JNIEnv, activity: JObject) -> Result<(), Box<dyn std::error::Error>> {
    let global_ref = env.new_global_ref(activity)?;
    let mut activity_guard = CURRENT_ACTIVITY.lock().unwrap();
    *activity_guard = Some(global_ref);
    Ok(())
}

#[cfg(target_os = "android")]
pub fn clear_current_activity() {
    let mut activity_guard = CURRENT_ACTIVITY.lock().unwrap();
    *activity_guard = None;
}

// JNI entry point to set the JavaVM
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _: *mut std::ffi::c_void) -> jint {
    JAVA_VM.set(vm).expect("Failed to set JavaVM");
    JNI_VERSION_1_6
}

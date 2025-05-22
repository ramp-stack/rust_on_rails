#[cfg(target_os = "android")]
use jni::objects::{JClass, JObject, JValue};
#[cfg(target_os = "android")]
use jni::JavaVM;
#[cfg(target_os = "android")]
use ndk_context::android_context;
#[cfg(target_os = "android")]
use std::error::Error;

#[cfg(target_os = "android")]
pub fn trigger_haptic_feedback() -> Result<(), Box<dyn Error>> {
    // Obtain the JavaVM pointer from ndk_context
    let android_ctx = android_context();
    let vm = unsafe { JavaVM::from_raw(android_ctx.vm() as *mut _) }?;
    let mut env = vm.attach_current_thread()?;

    // Obtain the android Context object from the native activity
    let context = unsafe {
        JObject::from_raw(*(android_ctx.context() as *mut jni::sys::jobject))
    };

    // Get the VIBRATOR_SERVICE string constant
    let vibrator_service_str = env.get_static_field(
        "android/content/Context",
        "VIBRATOR_SERVICE",
        "Ljava/lang/String;",
    )?.l()?;

    // Call getSystemService(Context.VIBRATOR_SERVICE)
    let vibrator_obj = env.call_method(
        context,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[JValue::Object(&vibrator_service_str)],
    )?.l()?;

    // Call vibrate(100) on the vibrator object (100 ms)
    env.call_method(
        vibrator_obj,
        "vibrate",
        "(J)V",
        &[JValue::Long(100)],
    )?;

    Ok(())
}

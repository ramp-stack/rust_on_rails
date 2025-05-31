#[cfg(target_os = "android")]
use jni::objects::JObject;
#[cfg(target_os = "android")]
use jni::JNIEnv;

#[cfg(target_os = "android")]
pub fn set_clipboard(env: &mut JNIEnv, context: JObject, text: &str) -> jni::errors::Result<()> {
    let clipboard_service = env
        .call_method(
            context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[(&env.new_string("clipboard")?).into()],
        )?
        .l()?;

    let clipboard_manager = JObject::from(clipboard_service);

    let clip_data_class = env.find_class("android/content/ClipData")?;
    let label = env.new_string("label")?;
    let text = env.new_string(text)?;
    let clip_data = env.call_static_method(
        clip_data_class,
        "newPlainText",
        "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
        &[(&JObject::from(label)).into(), (&JObject::from(text)).into()],
    )?.l()?;

    env.call_method(
        clipboard_manager,
        "setPrimaryClip",
        "(Landroid/content/ClipData;)V",
        &[(&clip_data).into()],
    )?;

    Ok(())
}

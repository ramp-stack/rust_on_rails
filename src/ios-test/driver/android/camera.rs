use crate::components::Drawable;
#[cfg(target_os = "android")]
use image::{Rgba, RgbaImage};
use jni::objects::{JClass, JPrimitiveArray};
use jni::sys::{jint, jobject};
#[cfg(target_os = "android")]
use jni::{
    objects::{GlobalRef, JByteBuffer, JObject, JObjectArray, JString, JValue},
   JNIEnv, JavaVM,
};
use jni_min_helper::*;

#[cfg(target_os = "android")]
use ndk_context::android_context;
#[cfg(target_os = "android")]
use std::error::Error;

#[cfg(target_os = "android")]
#[derive(Debug)]
pub struct AndroidCamera {
    java_vm: JavaVM,
    app_context: GlobalRef,
    camera_manager: GlobalRef,
    camera_state_callback: Option<GlobalRef>,
    image_width: i32,
    image_height: i32,
    image_format: i32,
    max_images: i32,
    camera_helper_class_loader: Option<GlobalRef>,
    camera_helper_instance: Option<GlobalRef>,
}

#[cfg(target_os = "android")]
impl AndroidCamera {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let jvm = unsafe { JavaVM::from_raw(android_context().vm() as *mut _)? };

        let (global_context, global_camera_manager) = {
            let mut env = jvm.attach_current_thread()?;

            let ctx_ptr = android_context().context();
            if ctx_ptr.is_null() {
                return Err("Failed to get Android context".into());
            }

            let context_obj = unsafe { JObject::from_raw(ctx_ptr as jobject) };
            let global_context = env.new_global_ref(context_obj)?;
            let global_camera_manager =
                Self::initialize_camera_manager_static(&mut env, &global_context)?;

            (global_context, global_camera_manager)
        };

        Ok(Self {
            java_vm: jvm,
            app_context: global_context,
            camera_manager: global_camera_manager,
            camera_state_callback: None,
            image_width: 1280,
            image_height: 720,
            image_format: 35,
            max_images: 2,
            camera_helper_class_loader: None,
            camera_helper_instance: None,
        })
    }

    fn initialize_camera_manager_static(
        env: &mut JNIEnv,
        context: &GlobalRef,
    ) -> Result<GlobalRef, Box<dyn Error>> {
        let camera_service = env
            .get_static_field("android/content/Context", "CAMERA_SERVICE", "Ljava/lang/String;")?
            .l()?;

        let manager = env
            .call_method(
                context.as_obj(),
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&camera_service)],
            )?
            .l()?;

        Ok(env.new_global_ref(manager)?)
    }

    unsafe fn copy_dex(&mut self) -> Result<(), Box<dyn Error>> {
        {
            let mut env = self.java_vm.attach_current_thread()?;
            println!("Starting to copy dex file.");

            let code_cache_dir_obj = env
                .call_method(self.app_context.as_obj(), "getCodeCacheDir", "()Ljava/io/File;", &[])?
                .l()?;
            let abs_path_obj = env
                .call_method(code_cache_dir_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])?
                .l()?;
            let code_cache_path: String = env.get_string(&JString::from(abs_path_obj))?.into();
            println!("Code cache path: {}", code_cache_path);

            let secondary_dex_path = format!("{}/secondary-dexes", code_cache_path);
            let file_class = env.find_class("java/io/File")?;
            let secondary_dex_jstr = env.new_string(&secondary_dex_path)?;
            let secondary_dex_file_obj = env.new_object(file_class, "(Ljava/lang/String;)V", &[JValue::Object(&secondary_dex_jstr)])?;
            env.call_method(secondary_dex_file_obj, "mkdirs", "()Z", &[])?;
            println!("Ensured secondary-dexes directory exists.");

            let asset_manager_obj = env
                .call_method(self.app_context.as_obj(), "getAssets", "()Landroid/content/res/AssetManager;", &[])?
                .l()?;
            let asset_name_jstr = env.new_string("classes.dex")?;
            let input_stream_obj = env
                .call_method(asset_manager_obj, "open", "(Ljava/lang/String;)Ljava/io/InputStream;", &[JValue::Object(&asset_name_jstr)])?
                .l()?;
            println!("Opened input stream for 'classes.dex'.");

            let dest_file_path = format!("{}/classes.dex", secondary_dex_path);
            let dest_file_jstr = env.new_string(&dest_file_path)?;
            let fos_class = env.find_class("java/io/FileOutputStream")?;
            let file_output_stream_obj = env.new_object(fos_class, "(Ljava/lang/String;)V", &[JValue::Object(&dest_file_jstr)])?;
            println!("Prepared output stream for destination file.");

            let buffer_size = 4096;
            let buffer = env.new_byte_array(buffer_size)?;

            loop {
                let bytes_read = env.call_method(&input_stream_obj, "read", "([B)I", &[JValue::Object(&buffer)])?.i()?;
                if bytes_read == -1 {
                    break;
                }
                env.call_method(
                    &file_output_stream_obj,
                    "write",
                    "([BII)V",
                    &[JValue::Object(&buffer), JValue::Int(0), JValue::Int(bytes_read)],
                )?;
            }

            env.call_method(input_stream_obj, "close", "()V", &[])?;
            env.call_method(file_output_stream_obj, "close", "()V", &[])?;
            println!("Dex file copied successfully to {}", dest_file_path);
        }

        self.dex_loader()?;
        Ok(())
    }


    unsafe fn dex_loader(&mut self) -> Result<(), Box<dyn Error>> {
        let mut env = self.java_vm.attach_current_thread()?;
        println!("Starting dex_loader");

        let dex_path = "/data/user/0/com.orange.pkg/code_cache/secondary-dexes/classes.dex";
        let optimized_dir = "/data/user/0/com.orange.pkg/code_cache";

        let dex_path_java = env.new_string(dex_path)?;
        println!("Dex path set: {}", dex_path);
        let optimized_dir_java = env.new_string(optimized_dir)?;
        println!("Optimized directory set: {}", optimized_dir);
        let null_str = JObject::null();

        let context_class = env.get_object_class(&self.app_context)?;
        println!("Retrieved context class: {:?}", context_class);
        let get_class_loader_method = env.get_method_id(context_class, "getClassLoader", "()Ljava/lang/ClassLoader;")?;
        println!("Retrieved getClassLoader method ID: {:?}", get_class_loader_method);
        let parent_class_loader = env.call_method_unchecked(
            &self.app_context,
            get_class_loader_method,
            jni::signature::ReturnType::Object,
            &[],
        )?.l()?;
        println!("Parent class loader obtained: {:?}", parent_class_loader);

        let dex_class_loader_class = env.find_class("dalvik/system/DexClassLoader")?;
        println!("DexClassLoader class found: {:?}", dex_class_loader_class);
        let constructor_id = env.get_method_id(
            &dex_class_loader_class,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/ClassLoader;)V",
        )?;
        println!("DexClassLoader constructor ID retrieved: {:?}", constructor_id);

        let dex_class_loader_obj = env.new_object_unchecked(
            dex_class_loader_class,
            constructor_id,
            &[
                JValue::Object(&dex_path_java).as_jni(),
                JValue::Object(&optimized_dir_java).as_jni(),
                JValue::Object(&null_str).as_jni(),
                JValue::Object(&parent_class_loader).as_jni(),
            ],
        )?;
        println!("DexClassLoader instantiated: {:?}", dex_class_loader_obj);

        self.camera_helper_class_loader = Some(env.new_global_ref(dex_class_loader_obj)?);

        let thread = env.call_static_method("java/lang/Thread", "currentThread", "()Ljava/lang/Thread;", &[])?.l()?;
        println!("Current thread obtained: {:?}", thread);
        env.call_method(
            thread,
            "setContextClassLoader",
            "(Ljava/lang/ClassLoader;)V",
            &[JValue::Object(self.camera_helper_class_loader.as_ref().unwrap().as_obj())],
        )?;
        println!("Context class loader set.");

        let class_name = env.new_string("com.orangeme.camera.CameraHelper")?;
        let camera_helper_class = env.call_method(
            self.camera_helper_class_loader.as_ref().unwrap().as_obj(),
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name)],
        )?.l()?;
        println!("CameraHelper class loaded: {:?}", camera_helper_class);

        let camera_helper_class_jclass = JClass::from(camera_helper_class);
        println!("Retrieved CameraHelper JClass: {:?}", camera_helper_class_jclass);
        let camera_helper_constructor = env.get_method_id(
            &camera_helper_class_jclass,
            "<init>",
            "(Landroid/content/Context;)V",
        )?;
        println!("CameraHelper constructor ID retrieved: {:?}", camera_helper_constructor);

        let camera_helper_obj = env.new_object_unchecked(
            camera_helper_class_jclass,
            camera_helper_constructor,
            &[JValue::Object(&self.app_context).as_jni()],
        )?;
        println!("CameraHelper instance created: {:?}", camera_helper_obj);

        self.camera_helper_instance = Some(env.new_global_ref(camera_helper_obj)?);

        Ok(())
    }

    pub fn open_camera(&mut self) -> Result<(), Box<dyn Error>> {
        unsafe { self.copy_dex().expect("error"); }
        println!("Opening camera using CameraHelper");

        let mut env = self.java_vm.attach_current_thread()?;
        println!("Attached to current Java thread.");

        // Obtain the primary camera ID using CameraHelper's getCameraIdList method
        let camera_helper = self.camera_helper_instance.as_ref()
            .ok_or("CameraHelper instance not initialized")?;
        println!("CameraHelper instance found: {:?}", camera_helper);

        let camera_helper_class = env.get_object_class(camera_helper.as_obj())?;
        println!("Retrieved CameraHelper class: {:?}", camera_helper_class);

        let camera_id_list_obj = env.call_method(
            camera_helper.as_obj(),
            "getCameraIdList",
            "()[Ljava/lang/String;",
            &[],
        )?.l()?;
        println!("Fetched camera ID list object: {:?}", camera_id_list_obj);

        let camera_id_array = JObjectArray::from(camera_id_list_obj);
        let camera_id_array_length = env.get_array_length(&camera_id_array)?;
        println!("Number of cameras found: {}", camera_id_array_length);
        if camera_id_array_length == 0 {
            return Err("No cameras found".into());
        }

        let first_camera_id_obj = env.get_object_array_element(&camera_id_array, 0)?;
        println!("First camera ID object: {:?}", first_camera_id_obj);
        let first_camera_id_jstr = JString::from(first_camera_id_obj);
        let camera_id: String = env.get_string(&first_camera_id_jstr)?.into();
        println!("Primary camera ID from CameraHelper: {}", camera_id);

        let j_camera_id = env.new_string(camera_id)?;

        let callback = self.camera_state_callback.as_ref()
            .ok_or("Camera state callback not initialized")?;
        println!("Camera state callback retrieved: {:?}", callback);

        env.call_method(
            camera_helper.as_obj(),
            "openCamera",
            "(Ljava/lang/String;Landroid/hardware/camera2/CameraDevice$StateCallback;)V",
            &[
                JValue::Object(&j_camera_id),
                JValue::Object(callback.as_obj()),
            ],
        )?;
        println!("OpenCamera method called successfully.");

        println!("Camera successfully opened.");

        Ok(())
    }

    fn create_image_reader(&self) -> Result<JObject<'_>, Box<dyn Error>> {
        let mut env = self.java_vm.attach_current_thread()?;
        let class = env.find_class("android/media/ImageReader")?;
        let reader = env.call_static_method(
            class,
            "newInstance",
            "(IIII)Landroid/media/ImageReader;",
            &[
                JValue::Int(self.image_width),
                JValue::Int(self.image_height),
                JValue::Int(self.image_format),
                JValue::Int(self.max_images),
            ],
        )?.l()?;

        Ok(reader)
    }

    fn acquire_image(&self, reader: &JObject) -> Result<JObject<'_>, Box<dyn Error>> {
        let mut env = self.java_vm.attach_current_thread()?;
        Ok(env
            .call_method(reader, "acquireNextImage", "()Landroid/media/Image;", &[])?
            .l()?)
    }

    fn get_image_dimensions(&self, image: &JObject) -> Result<(i32, i32), Box<dyn Error>> {
        let mut env = self.java_vm.attach_current_thread()?;
        let width = env.call_method(image, "getWidth", "()I", &[])?.i()?;
        let height = env.call_method(image, "getHeight", "()I", &[])?.i()?;
        Ok((width, height))
    }

    fn convert_yuv_to_rgba(
        &self,
        image: &JObject,
        width: i32,
        height: i32,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut env = self.java_vm.attach_current_thread()?;

        let planes: JObjectArray = env
            .call_method(image, "getPlanes", "()[Landroid/media/Image$Plane;", &[])?
            .l()?
            .into();

        if env.get_array_length(&planes)? < 3 {
            return Err("Image does not have the expected YUV planes".into());
        }

        let mut extract = |idx| -> Result<(Vec<u8>, i32, i32), Box<dyn Error>> {
            let plane = env.get_object_array_element(&planes, idx)?;
            let buffer = env.call_method(&plane, "getBuffer", "()Ljava/nio/ByteBuffer;", &[])?.l()?;
            let byte_buffer = JByteBuffer::from(buffer);

            let len = env.get_direct_buffer_capacity(&byte_buffer)?;
            let ptr = env.get_direct_buffer_address(&byte_buffer)?;
            let data = unsafe { std::slice::from_raw_parts(ptr, len).to_vec() };

            let row_stride = env.call_method(&plane, "getRowStride", "()I", &[])?.i()?;
            let pixel_stride = env.call_method(&plane, "getPixelStride", "()I", &[])?.i()?;

            Ok((data, row_stride, pixel_stride))
        };

        let (y, y_rs, y_ps) = extract(0)?;
        let (u, u_rs, u_ps) = extract(1)?;
        let (v, v_rs, v_ps) = extract(2)?;

        let mut rgba = Vec::with_capacity((width * height * 4) as usize);

        for row in 0..height {
            for col in 0..width {
                let yi = (row * y_rs + col * y_ps) as usize;
                let ui = ((row / 2) * u_rs + (col / 2) * u_ps) as usize;
                let vi = ((row / 2) * v_rs + (col / 2) * v_ps) as usize;

                let y_val = y.get(yi).copied().unwrap_or(0) as i32;
                let u_val = u.get(ui).copied().unwrap_or(128) as i32;
                let v_val = v.get(vi).copied().unwrap_or(128) as i32;

                let c = y_val - 16;
                let d = u_val - 128;
                let e = v_val - 128;

                let r = ((298 * c + 409 * e + 128) >> 8).clamp(0, 255) as u8;
                let g = ((298 * c - 100 * d - 208 * e + 128) >> 8).clamp(0, 255) as u8;
                let b = ((298 * c + 516 * d + 128) >> 8).clamp(0, 255) as u8;

                rgba.extend_from_slice(&[r, g, b, 255]);
            }
        }

        Ok(rgba)
    }

    pub fn get_latest_frame(&self) -> Result<RgbaImage, Box<dyn Error>> {
        println!("get_latest_frame");
        println!("start get_latest_frame");
        let mut env = self.java_vm.attach_current_thread()?;
        let reader = self.create_image_reader()?;
        let image = self.acquire_image(&reader)?;
        let (w, h) = self.get_image_dimensions(&image)?;
        let data = self.convert_yuv_to_rgba(&image, w, h)?;

        let mut img = RgbaImage::new(w as u32, h as u32);

        for (i, px) in data.chunks_exact(4).enumerate() {
            let x = (i % w as usize) as u32;
            let y = (i / w as usize) as u32;
            img.put_pixel(x, y, Rgba([px[0], px[1], px[2], px[3]]));
        }

        env.call_method(&image, "close", "()V", &[])?;
        Ok(img)
    }
}











// ___           _     _
// / _ \         | |   (_)
// / /_\ \_ __ ___| |__  ___   _____  ___
// |  _  | '__/ __| '_ \| \ \ / / _ \/ __|
// | | | | | | (__| | | | |\ V /  __/\__ \
// \_| |_/_|  \___|_| |_|_| \_/ \___||___/
//








//
// unsafe fn check_asset_dex_file(&self, env: &mut JNIEnv) -> Result<(), ()> {
//     let asset_manager = env
//         .call_method(
//             self.app_context.as_obj(),
//             "getAssets",
//             "()Landroid/content/res/AssetManager;",
//             &[],
//         )
//         .and_then(|r| r.l())
//         .map_err(|_| {
//             println!("Failed to get AssetManager");
//         })?;
//
//     let asset_name = env.new_string("classes.dex").map_err(|_| {
//         println!("Failed to create JString for asset name");
//     })?;
//
//     let asset_open_result = env.call_method(
//         asset_manager,
//         "open",
//         "(Ljava/lang/String;)Ljava/io/InputStream;",
//         &[JValue::Object(&JObject::from(asset_name))],
//     );
//
//     match asset_open_result {
//         Ok(input_stream_obj) => {
//             let input_stream = input_stream_obj.l();
//             match input_stream {
//                 Ok(_) => println!("Asset 'classes.dex' exists in assets."),
//                 Err(_) => println!("Asset 'classes.dex' does NOT exist in assets."),
//             }
//         }
//         Err(_) => {
//             println!("Asset 'classes.dex' does NOT exist in assets.");
//         }
//     }
//
//     Ok(())
// }















//
// unsafe fn list_dex_and_class_in_assets(&self, env: &mut JNIEnv) -> Result<(), ()> {
//     println!("Listing .dex/.class files in assets:");
//
//     // Get AssetManager object
//     let asset_manager = env
//         .call_method(
//             self.app_context.as_obj(),
//             "getAssets",
//             "()Landroid/content/res/AssetManager;",
//             &[],
//         )
//         .and_then(|r| r.l())
//         .map_err(|_| {
//             println!("Failed to get AssetManager for listing.");
//         })?;
//
//     // Create an empty string for root path
//     let root_path = env.new_string("").map_err(|_| {
//         println!("Failed to create root path string.");
//     })?;
//
//     // Call AssetManager.list("")
//     let asset_list_obj = env
//         .call_method(
//             asset_manager,
//             "list",
//             "(Ljava/lang/String;)[Ljava/lang/String;",
//             &[jni::objects::JValue::Object(&JObject::from(root_path))],
//         )
//         .and_then(|r| r.l())
//         .map_err(|_| {
//             println!("Failed to list assets.");
//         })?;
//
//     // Cast returned object to JObjectArray explicitly
//     let array = JObjectArray::from(asset_list_obj);
//
//     // Get array length
//     let array_len = env.get_array_length(&array).unwrap_or(0);
//
//     for i in 0..array_len {
//         // Get element at index i
//         let string_obj = env.get_object_array_element(&array, i).map_err(|_| {
//             println!("Failed to get element at index {}", i);
//         })?;
//
//         // Convert JObject to JString
//         let jstr = JString::from(string_obj);
//
//         // Get Rust string from JString safely
//         let rust_str = env.get_string(&jstr).map_err(|_| {
//             println!("Failed to convert JString to Rust string");
//         })?;
//
//         let file_name = rust_str.to_string_lossy();
//
//         if file_name.ends_with(".dex") || file_name.ends_with(".class") {
//             println!("Found asset file: {}", file_name);
//         }
//     }
//
//     Ok(())
// }





// unsafe fn dex_loader(&mut self) -> Result<(), Box<dyn Error>> {
//     let mut env = self.java_vm.attach_current_thread()?;
//     println!("Starting dex_loader");
//
//     let dex_path = "/data/user/0/com.orange.pkg/code_cache/secondary-dexes/classes.dex";
//     let optimized_dir = "/data/user/0/com.orange.pkg/code_cache";
//
//     let dex_path_java = env.new_string(dex_path)?;
//     let optimized_dir_java = env.new_string(optimized_dir)?;
//     let null_str: JObject = JObject::null();
//
//     let context_class = env.get_object_class(&self.app_context)?;
//     let get_class_loader_method = env.get_method_id(context_class, "getClassLoader", "()Ljava/lang/ClassLoader;")?;
//     let parent_class_loader = env.call_method_unchecked(
//         &self.app_context,
//         get_class_loader_method,
//         jni::signature::ReturnType::Object,
//         &[],
//     )?.l()?;
//
//     let dex_class_loader_class = env.find_class("dalvik/system/DexClassLoader")?;
//     let constructor_id = env.get_method_id(
//         &dex_class_loader_class,
//         "<init>",
//         "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/ClassLoader;)V",
//     )?;
//
//     let dex_class_loader_obj = env.new_object_unchecked(
//         dex_class_loader_class,
//         constructor_id,
//         &[
//             JValue::Object(&dex_path_java).as_jni(),
//             JValue::Object(&optimized_dir_java).as_jni(),
//             JValue::Object(&null_str).as_jni(),
//             JValue::Object(&parent_class_loader).as_jni(),
//         ],
//     )?;
//
//     println!("DexClassLoader instantiated.");
//
//     let to_string = env.call_method(
//         &dex_class_loader_obj,
//         "toString",
//         "()Ljava/lang/String;",
//         &[],
//     )?.l()?;
//     let to_string_str: String = env.get_string(&JString::from(to_string))?.into();
//     println!("DexClassLoader toString(): {}", to_string_str);
//
//
//     let thread = env.call_static_method("java/lang/Thread", "currentThread", "()Ljava/lang/Thread;", &[])?.l()?;
//     env.call_method(thread, "setContextClassLoader", "(Ljava/lang/ClassLoader;)V", &[JValue::Object(&dex_class_loader_obj)])?;
//     println!("Context class loader set.");
//
//     let class_name = env.new_string("com.orangeme.camera.CameraHelper")?;
//     let class = env.call_method(
//         &dex_class_loader_obj,
//         "loadClass",
//         "(Ljava/lang/String;)Ljava/lang/Class;",
//         &[JValue::Object(&class_name)],
//     )?.l()?;
//     println!("Class loaded via DexClassLoader: com.orangeme.camera.CameraHelper");
//
//     let name_obj = env.call_method(&class, "getName", "()Ljava/lang/String;", &[])?.l()?;
//     let class_name_str: String = env.get_string(&JString::from(name_obj))?.into();
//     println!("Class name: {}", class_name_str);
//
//     let superclass_obj = env.call_method(&class, "getSuperclass", "()Ljava/lang/Class;", &[])?.l()?;
//     if !superclass_obj.is_null() {
//         let superclass_name_obj = env.call_method(superclass_obj, "getName", "()Ljava/lang/String;", &[])?.l()?;
//         let superclass_name_str: String = env.get_string(&JString::from(superclass_name_obj))?.into();
//         println!("Superclass: {}", superclass_name_str);
//     } else {
//         println!("Superclass: None");
//     }
//
//     let interfaces_array = env.call_method(&class, "getInterfaces", "()[Ljava/lang/Class;", &[])?.l()?;
//     let interfaces: JObjectArray = JObjectArray::from(interfaces_array);
//     let count = env.get_array_length(&interfaces)?;
//     println!("Interfaces ({}):", count);
//     for i in 0..count {
//         let iface_obj = env.get_object_array_element(&interfaces, i)?;
//         let iface_name_obj = env.call_method(iface_obj, "getName", "()Ljava/lang/String;", &[])?.l()?;
//         let iface_name_str: String = env.get_string(&JString::from(iface_name_obj))?.into();
//         println!(" - {}", iface_name_str);
//     }
//
//     let methods_array = env.call_method(&class, "getDeclaredMethods", "()[Ljava/lang/reflect/Method;", &[])?.l()?;
//     let methods: JObjectArray = JObjectArray::from(methods_array);
//     let method_count = env.get_array_length(&methods)?;
//     println!("Declared Methods ({}):", method_count);
//     for i in 0..method_count {
//         let method_obj = env.get_object_array_element(&methods, i)?;
//         let method_str_obj = env.call_method(method_obj, "toString", "()Ljava/lang/String;", &[])?.l()?;
//         let method_str: String = env.get_string(&JString::from(method_str_obj))?.into();
//         println!(" - {}", method_str);
//     }
//
//     let fields_array = env.call_method(&class, "getDeclaredFields", "()[Ljava/lang/reflect/Field;", &[])?.l()?;
//     let fields: JObjectArray = JObjectArray::from(fields_array);
//     let field_count = env.get_array_length(&fields)?;
//     println!("Declared Fields ({}):", field_count);
//     for i in 0..field_count {
//         let field_obj = env.get_object_array_element(&fields, i)?;
//         let field_str_obj = env.call_method(field_obj, "toString", "()Ljava/lang/String;", &[])?.l()?;
//         let field_str: String = env.get_string(&JString::from(field_str_obj))?.into();
//         println!(" - {}", field_str);
//     }
//
//     Ok(())
// }











// unsafe fn scan_internal_dex_paths(&self) -> Result<(), Box<dyn Error>>  {
//     let mut env = self.java_vm.attach_current_thread()?;
//     println!("Searching internal directories for .dex/.odex/.vdex files:");
//
//     let code_cache_path: Option<String> = env
//         .call_method(self.app_context.as_obj(), "getCodeCacheDir", "()Ljava/io/File;", &[])
//         .ok()
//         .and_then(|r| r.l().ok())
//         .and_then(|dir_obj| {
//             env.call_method(dir_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])
//                 .ok()
//                 .and_then(|s| s.l().ok())
//                 .and_then(|path_obj| {
//                     let jstr = JString::from(path_obj);
//                     env.get_string(&jstr)
//                         .ok()
//                         .map(|jni_str| jni_str.into())
//                 })
//         });
//
//     let files_path: Option<String> = env
//         .call_method(self.app_context.as_obj(), "getFilesDir", "()Ljava/io/File;", &[])
//         .ok()
//         .and_then(|r| r.l().ok())
//         .and_then(|dir_obj| {
//             env.call_method(dir_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])
//                 .ok()
//                 .and_then(|s| s.l().ok())
//                 .and_then(|path_obj| {
//                     let jstr = JString::from(path_obj);
//                     env.get_string(&jstr)
//                         .ok()
//                         .map(|jni_str| jni_str.into())
//                 })
//         });
//
//     let mut paths_to_check: Vec<String> = vec![];
//
//     if let Some(code_cache) = &code_cache_path {
//         println!("Code cache path: {}", code_cache);
//         paths_to_check.push(code_cache.clone());
//         paths_to_check.push(format!("{}/secondary-dexes", code_cache));
//     }
//
//     if let Some(files) = &files_path {
//         println!("Files path: {}", files);
//         paths_to_check.push(files.clone());
//         paths_to_check.push(format!("{}/oat/arm", files));
//     }
//
//     for path in &paths_to_check {
//         if let Ok(entries) = fs::read_dir(&path) {
//             for entry in entries.flatten() {
//                 if let Ok(file_name) = entry.file_name().into_string() {
//                     if file_name.ends_with(".dex")
//                         || file_name.ends_with(".odex")
//                         || file_name.ends_with(".vdex")
//                     {
//                         println!("Found file in {}: {}", path, file_name);
//                     }
//                 }
//             }
//         }
//     }
//
//     Ok(())
// }










// unsafe fn copy_dex(&mut self) -> Result<(), Box<dyn Error>> {
//     {
//         let mut env = self.java_vm.attach_current_thread()?;
//         println!("Starting to copy dex file.");
//
//         println!("Getting code cache directory.");
//         let code_cache_dir_obj = env
//             .call_method(self.app_context.as_obj(), "getCodeCacheDir", "()Ljava/io/File;", &[])?
//             .l()?;
//         println!("Retrieved code cache directory object.");
//
//         let abs_path_obj = env
//             .call_method(code_cache_dir_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])?
//             .l()?;
//         println!("Retrieved absolute path object for code cache directory.");
//         let abs_path_jstr = JString::from(abs_path_obj);
//         let code_cache_path: String = env.get_string(&abs_path_jstr)?.into();
//         println!("Code cache path: {}", code_cache_path);
//
//         let secondary_dex_path = format!("{}/secondary-dexes", code_cache_path);
//         println!("Secondary dex path: {}", secondary_dex_path);
//
//         println!("Creating secondary-dexes directory if not present.");
//         let file_class = env.find_class("java/io/File")?;
//         let secondary_dex_jstr = env.new_string(&secondary_dex_path)?;
//         let secondary_dex_file_obj = env.new_object(
//             file_class,
//             "(Ljava/lang/String;)V",
//             &[JValue::Object(&secondary_dex_jstr)],
//         )?;
//         env.call_method(secondary_dex_file_obj, "mkdirs", "()Z", &[])?;
//         println!("Secondary-dexes directory created.");
//
//         println!("Accessing AssetManager to open 'classes.dex'.");
//         let asset_manager_obj = env
//             .call_method(self.app_context.as_obj(), "getAssets", "()Landroid/content/res/AssetManager;", &[])?
//             .l()?;
//         let asset_name_jstr = env.new_string("classes.dex")?;
//         let input_stream_obj = env
//             .call_method(
//                 asset_manager_obj,
//                 "open",
//                 "(Ljava/lang/String;)Ljava/io/InputStream;",
//                 &[JValue::Object(&asset_name_jstr)],
//             )?
//             .l()?;
//         println!("Successfully opened input stream for 'classes.dex'.");
//
//         println!("Preparing file output stream for destination file.");
//         let dest_file_path = format!("{}/classes.dex", secondary_dex_path);
//         println!("Destination file path: {}", dest_file_path);
//         let dest_file_jstr = env.new_string(&dest_file_path)?;
//         let fos_class = env.find_class("java/io/FileOutputStream")?;
//         let file_output_stream_obj = env.new_object(
//             fos_class,
//             "(Ljava/lang/String;)V",
//             &[JValue::Object(&dest_file_jstr)],
//         )?;
//         println!("File output stream for destination file prepared.");
//
//         println!("Preparing to copy data.");
//         let buffer_size = 4096;
//         let buffer = env.new_byte_array(buffer_size)?;
//         println!("Buffer of size {} created.", buffer_size);
//
//         let input_stream_class = env.get_object_class(&input_stream_obj)?;
//         let read_method_id = env.get_method_id(input_stream_class, "read", "([B)I")?;
//         println!("Read method ID retrieved.");
//
//         let fos_class_obj = env.get_object_class(&file_output_stream_obj)?;
//         let write_method_id = env.get_method_id(fos_class_obj, "write", "([BII)V")?;
//         println!("Write method ID retrieved.");
//
//         loop {
//             let bytes_read = env
//                 .call_method(
//                     &input_stream_obj,
//                     "read",
//                     "([B)I",
//                     &[JValue::Object(&buffer)],
//                 )?
//                 .i()?;
//
//             if bytes_read == -1 {
//                 println!("End of input stream reached.");
//                 break;
//             }
//
//             println!("Read {} bytes from input stream.", bytes_read);
//
//             env.call_method(
//                 &file_output_stream_obj,
//                 "write",
//                 "([BII)V",
//                 &[
//                     JValue::Object(&buffer),
//                     JValue::Int(0),
//                     JValue::Int(bytes_read),
//                 ],
//             )?;
//             println!("Written {} bytes to output stream.", bytes_read);
//         }
//
//         env.call_method(input_stream_obj, "close", "()V", &[])?;
//         println!("Input stream closed.");
//         env.call_method(file_output_stream_obj, "close", "()V", &[])?;
//         println!("File output stream closed.");
//
//         println!("Dex file copy completed successfully.");
//     }
//
//     println!("Scanning internal directories for .dex files:");
//     self.scan_internal_dex_paths().expect("error");
//
//     Ok(())
// }







// unsafe fn dex_loader(&mut self) -> Result<(), Box<dyn Error>> {
//     let mut env = self.java_vm.attach_current_thread()?;
//     println!("Starting dex_loader");
//
//     let dex_path = "/data/user/0/com.orange.pkg/code_cache/secondary-dexes/classes.dex";
//     let optimized_dir = "/data/user/0/com.orange.pkg/code_cache";
//
//     let dex_path_java = env.new_string(dex_path)?;
//     println!("Dex path set: {}", dex_path);
//     let optimized_dir_java = env.new_string(optimized_dir)?;
//     println!("Optimized directory set: {}", optimized_dir);
//     let null_str = JObject::null();
//
//     let context_class = env.get_object_class(&self.app_context)?;
//     println!("Retrieved context class: {:?}", context_class);
//     let get_class_loader_method = env.get_method_id(context_class, "getClassLoader", "()Ljava/lang/ClassLoader;")?;
//     println!("Retrieved getClassLoader method ID: {:?}", get_class_loader_method);
//     let parent_class_loader = env.call_method_unchecked(
//         &self.app_context,
//         get_class_loader_method,
//         jni::signature::ReturnType::Object,
//         &[],
//     )?.l()?;
//     println!("Parent class loader obtained: {:?}", parent_class_loader);
//
//     let dex_class_loader_class = env.find_class("dalvik/system/DexClassLoader")?;
//     println!("DexClassLoader class found: {:?}", dex_class_loader_class);
//     let constructor_id = env.get_method_id(
//         &dex_class_loader_class,
//         "<init>",
//         "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/ClassLoader;)V",
//     )?;
//     println!("DexClassLoader constructor ID retrieved: {:?}", constructor_id);
//
//     let dex_class_loader_obj = env.new_object_unchecked(
//         dex_class_loader_class,
//         constructor_id,
//         &[
//             JValue::Object(&dex_path_java).as_jni(),
//             JValue::Object(&optimized_dir_java).as_jni(),
//             JValue::Object(&null_str).as_jni(),
//             JValue::Object(&parent_class_loader).as_jni(),
//         ],
//     )?;
//
//
//     println!("DexClassLoader instantiated: {:?}", dex_class_loader_obj);
//
//     let thread = env.call_static_method("java/lang/Thread", "currentThread", "()Ljava/lang/Thread;", &[])?.l()?;
//     println!("Current thread obtained: {:?}", thread);
//     env.call_method(thread, "setContextClassLoader", "(Ljava/lang/ClassLoader;)V", &[JValue::Object(&dex_class_loader_obj)])?;
//     println!("Context class loader set.");
//
//     let class_name = env.new_string("com.orangeme.camera.CameraHelper")?;
//     let camera_helper_class = env.call_method(
//         &dex_class_loader_obj,
//         "loadClass",
//         "(Ljava/lang/String;)Ljava/lang/Class;",
//         &[JValue::Object(&class_name)],
//     )?.l()?;
//     println!("CameraHelper class loaded: {:?}", camera_helper_class);
//
//     let camera_helper_class_jclass = JClass::from(camera_helper_class);
//     println!("Retrieved CameraHelper JClass: {:?}", camera_helper_class_jclass);
//     let camera_helper_constructor = env.get_method_id(
//         &camera_helper_class_jclass,
//         "<init>",
//         "(Landroid/content/Context;)V",
//     )?;
//     println!("CameraHelper constructor ID retrieved: {:?}", camera_helper_constructor);
//
//     let camera_helper_obj = env.new_object_unchecked(
//         camera_helper_class_jclass,
//         camera_helper_constructor,
//         &[JValue::Object(&self.app_context).as_jni()],
//     )?;
//     println!("CameraHelper instance created: {:?}", camera_helper_obj);
//
//     let camera_id_list_obj = env.call_method(
//         &camera_helper_obj,
//         "getCameraIdList",
//         "()[Ljava/lang/String;",
//         &[],
//     )?.l()?;
//     println!("Camera ID list obtained: {:?}", camera_id_list_obj);
//
//     let camera_id_array = JObjectArray::from(camera_id_list_obj);
//     let array_len = env.get_array_length(&camera_id_array)?;
//     println!("Number of camera IDs: {}", array_len);
//     let mut camera_ids = Vec::with_capacity(array_len as usize);
//     for i in 0..array_len {
//         let jstr_obj = env.get_object_array_element(&camera_id_array, i)?;
//         let jstr = JString::from(jstr_obj);
//         let rust_str: String = env.get_string(&jstr)?.into();
//         println!("Camera ID [{}]: {}", i, rust_str);
//         camera_ids.push(rust_str);
//     }
//     println!("Camera IDs: {:?}", camera_ids);
//
//     let camera_manager_obj = env.call_method(
//         &camera_helper_obj,
//         "getCameraManager",
//         "()Landroid/hardware/camera2/CameraManager;",
//         &[],
//     )?.l()?;
//     println!("CameraManager obtained: {:?}", camera_manager_obj);
//
//     Ok(())
// }







// unsafe fn camera_callback(&mut self) -> Result<(), jni::errors::Error> {
//     if self.camera_state_callback.is_none() {
//         self.copy_dex().expect("error");
//
//         let mut env = self.java_vm.attach_current_thread()?;
//
//         let callback_class = env.find_class("android/hardware/camera2/CameraDevice$StateCallback")?;
//         println!("class found: {:?}", callback_class);
//
//         let proxy = match JniProxy::build(
//             &mut env,
//             None,
//             &["android/hardware/camera2/CameraDevice$StateCallback"],
//             |_env, _method_obj, _args| {
//                 println!("proxy callback started");
//                 Ok(_env.auto_local(JObject::null()))
//             },
//         ) {
//             Ok(proxy) => proxy,
//             Err(e) => {
//                 error!("Failed to build JNI proxy: {:?}", e);
//                 return Err(e);
//             }
//         };
//
//         let global = env.new_global_ref(proxy)?;
//         self.camera_state_callback = Some(global);
//     }
//
//     Ok(())
// }






// fn primary_camera_id_with_env(&self, env: &mut JNIEnv) -> Result<String, Box<dyn Error>> {
//     let context_class = env.find_class("android/content/Context")?;
//     let camera_service_field = env.get_static_field(context_class, "CAMERA_SERVICE", "Ljava/lang/String;")?;
//     let camera_service_str = camera_service_field.l()?;
//
//     let camera_manager = env
//         .call_method(
//             self.app_context.as_obj(),
//             "getSystemService",
//             "(Ljava/lang/String;)Ljava/lang/Object;",
//             &[JValue::Object(&camera_service_str.into())],
//         )?
//         .l()?;
//
//     let camera_id_array = env
//         .call_method(
//             camera_manager,
//             "getCameraIdList",
//             "()[Ljava/lang/String;",
//             &[],
//         )?
//         .l()?;
//
//     let camera_ids = env.get_object_array_element(JObjectArray::from(camera_id_array), 0)?;
//     let camera_id_jstr = JString::from(camera_ids);
//     let camera_id: String = env.get_string(&camera_id_jstr)?.into();
//     println!("Primary camera id: {}", camera_id);
//     Ok(camera_id)
// }



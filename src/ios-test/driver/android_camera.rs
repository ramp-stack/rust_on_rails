#[cfg(target_os = "android")]
use image::{Rgba, RgbaImage};
#[cfg(target_os = "android")]
use jni::objects::{JByteBuffer, JObject, JObjectArray, JString, JValue};
#[cfg(target_os = "android")]
use jni::JNIEnv;
#[cfg(target_os = "android")]
use ndk_context::android_context;
#[cfg(target_os = "android")]
use std::convert::TryInto;
#[cfg(target_os = "android")]
use std::error::Error;

#[cfg(target_os = "android")]
pub struct AndroidCamera<'a> {
    app_context: JObject<'a>,
    env: Box<JNIEnv<'a>>,
    camera_manager: JObject<'a>,
    image_width: i32,
    image_height: i32,
    image_format: i32,
    max_images: i32,
}

#[cfg(target_os = "android")]
impl<'a> AndroidCamera<'a> {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let android_context_obj = android_context();
        let mut env = unsafe { JNIEnv::from_raw(android_context_obj.context() as *mut jni::sys::JNIEnv)? };
        let context = unsafe { JObject::from_raw(android_context_obj.context() as jni::sys::jobject) };

        let camera_service_string = env.get_static_field(
            "android/content/Context",
            "CAMERA_SERVICE",
            "Ljava/lang/String;",
        )?.l()?;

        let camera_manager = env.call_method(
            &context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&camera_service_string)],
        )?.l()?;

        let default_width = 1280;
        let default_height = 720;
        let default_format = 35;
        let default_max_images = 2;

        Ok(Self {
            app_context: context,
            env: Box::new(env),
            camera_manager,
            image_width: default_width,
            image_height: default_height,
            image_format: default_format,
            max_images: default_max_images,
        })
    }

    fn get_image_width(&self) -> Result<i32, Box<dyn Error>> {
        Ok(self.image_width)
    }

    fn get_image_height(&self) -> Result<i32, Box<dyn Error>> {
        Ok(self.image_height)
    }

    fn get_image_format(&self) -> Result<i32, Box<dyn Error>> {
        Ok(self.image_format)
    }

    fn get_max_images(&self) -> Result<i32, Box<dyn Error>> {
        Ok(self.max_images)
    }

    pub fn configure_image(&mut self, width: i32, height: i32, format: i32, max_images: i32) {
        self.image_width = width;
        self.image_height = height;
        self.image_format = format;
        self.max_images = max_images;
    }

    pub fn primary_camera_id(&mut self) -> Result<String, Box<dyn Error>> {
        let camera_id_array_obj: JObjectArray = JObjectArray::from(
            self.env.call_method(
                &self.camera_manager,
                "getCameraIdList",
                "()[Ljava/lang/String;",
                &[],
            )?.l()?
        );

        let length = self.env.get_array_length(&camera_id_array_obj)?;

        if length == 0 {
            return Err("No camera found".into());
        }

        let camera_id_obj = self.env.get_object_array_element(&camera_id_array_obj, 0)?;
        let camera_id_jstring = JString::from(camera_id_obj);
        let primary_camera_id = self.env.get_string(&camera_id_jstring)?.into();

        Ok(primary_camera_id)
    }

    pub fn open_camera(&mut self) -> Result<(), Box<dyn Error>> {
        let primary_camera_id = self.primary_camera_id()?;
        let primary_camera_string = self.env.new_string(primary_camera_id)?;

        let state_callback_class = self.env.find_class("android/hardware/camera2/CameraDevice$StateCallback")?;
        let state_callback = self.env.alloc_object(state_callback_class)?;

        self.env.call_method(
            &self.camera_manager,
            "openCamera",
            "(Ljava/lang/String;Landroid/hardware/camera2/CameraDevice$StateCallback;Landroid/os/Handler;)V",
            &[
                JValue::Object(&primary_camera_string),
                JValue::Object(&state_callback),
                JValue::Object(&JObject::null()),
            ],
        )?;

        Ok(())
    }

    pub fn capture_rgba_image(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let image_reader = self.create_image_reader()?;
        let image = self.acquire_image(&image_reader)?;

        let width = self.env.call_method(&image, "getWidth", "()I", &[])?.i()?;
        let height = self.env.call_method(&image, "getHeight", "()I", &[])?.i()?;

        let rgba_data = self.convert_yuv_to_rgba(&image, width, height)?;

        Ok(rgba_data)
    }

    fn get_image_dimensions(&mut self, image: &JObject<'a>) -> Result<(i32, i32), Box<dyn Error>> {
        let width = self.env.call_method(image, "getWidth", "()I", &[])?.i()?;
        let height = self.env.call_method(image, "getHeight", "()I", &[])?.i()?;
        Ok((width, height))
    }

    fn create_image_reader(&mut self) -> Result<JObject<'a>, Box<dyn Error>> {
        let image_reader_class = self.env.find_class("android/media/ImageReader")?;

        let image_reader = self.env.new_object(
            image_reader_class,
            "(IIII)V",
            &[
                JValue::Int(self.get_image_width()?),
                JValue::Int(self.get_image_height()?),
                JValue::Int(self.get_image_format()?),
                JValue::Int(self.get_max_images()?),
            ],
        )?;
        Ok(image_reader)
    }

    fn acquire_image(&mut self, image_reader: &JObject<'a>) -> Result<JObject<'a>, Box<dyn Error>> {
        let image = self.env.call_method(
            image_reader,
            "acquireNextImage",
            "()Landroid/media/Image;",
            &[],
        )?.l()?;
        Ok(image)
    }

    fn convert_yuv_to_rgba(&mut self, image: &JObject<'a>, image_width: i32, image_height: i32) -> Result<Vec<u8>, Box<dyn Error>> {
        let planes_array_object = self.env.call_method(
            image,
            "getPlanes",
            "()[Landroid/media/Image$Plane;",
            &[],
        )?.l()?;

        let planes_array = JObjectArray::from(planes_array_object);
        let array_length = self.env.get_array_length(&planes_array)?;

        if array_length < 3 {
            return Err("Image does not have the expected YUV planes".into());
        }

        let y_plane = self.env.get_object_array_element(&planes_array, 0)?;
        let u_plane = self.env.get_object_array_element(&planes_array, 1)?;
        let v_plane = self.env.get_object_array_element(&planes_array, 2)?;

        let y_buffer = self.env.call_method(
            &y_plane,
            "getBuffer",
            "()Ljava/nio/ByteBuffer;",
            &[],
        )?.l()?;

        let u_buffer = self.env.call_method(
            &u_plane,
            "getBuffer",
            "()Ljava/nio/ByteBuffer;",
            &[],
        )?.l()?;

        let v_buffer = self.env.call_method(
            &v_plane,
            "getBuffer",
            "()Ljava/nio/ByteBuffer;",
            &[],
        )?.l()?;

        let y_pixel_stride = self.env.call_method(&y_plane, "getPixelStride", "()I", &[])?.i()?;
        let y_row_stride = self.env.call_method(&y_plane, "getRowStride", "()I", &[])?.i()?;

        let u_pixel_stride = self.env.call_method(&u_plane, "getPixelStride", "()I", &[])?.i()?;
        let u_row_stride = self.env.call_method(&u_plane, "getRowStride", "()I", &[])?.i()?;

        let v_pixel_stride = self.env.call_method(&v_plane, "getPixelStride", "()I", &[])?.i()?;
        let v_row_stride = self.env.call_method(&v_plane, "getRowStride", "()I", &[])?.i()?;

        let y_buffer_size = self.env.get_direct_buffer_capacity(<&JByteBuffer>::from(&y_buffer))?;
        let u_buffer_size = self.env.get_direct_buffer_capacity(<&JByteBuffer>::from(&u_buffer))?;
        let v_buffer_size = self.env.get_direct_buffer_capacity(<&JByteBuffer>::from(&v_buffer))?;

        let y_data = self.env.get_direct_buffer_address(<&JByteBuffer>::from(&y_buffer))?;
        let u_data = self.env.get_direct_buffer_address(<&JByteBuffer>::from(&u_buffer))?;
        let v_data = self.env.get_direct_buffer_address(<&JByteBuffer>::from(&v_buffer))?;

        let y_slice = unsafe { std::slice::from_raw_parts(y_data, y_buffer_size) };
        let u_slice = unsafe { std::slice::from_raw_parts(u_data, u_buffer_size) };
        let v_slice = unsafe { std::slice::from_raw_parts(v_data, v_buffer_size) };

        let mut rgba_pixel_data = Vec::with_capacity((image_width * image_height * 4) as usize);

        for row in 0..image_height {
            for col in 0..image_width {
                let y_index = (row * y_row_stride + col * y_pixel_stride) as usize;

                let u_row = row / 2;
                let u_col = col / 2;
                let u_index = (u_row * u_row_stride + u_col * u_pixel_stride) as usize;

                let v_row = row / 2;
                let v_col = col / 2;
                let v_index = (v_row * v_row_stride + v_col * v_pixel_stride) as usize;

                let y_value = if y_index < y_buffer_size { y_slice[y_index] as i32 } else { 0 };
                let u_value = if u_index < u_buffer_size { u_slice[u_index] as i32 } else { 128 };
                let v_value = if v_index < v_buffer_size { v_slice[v_index] as i32 } else { 128 };

                let c = y_value - 16;
                let d = u_value - 128;
                let e = v_value - 128;

                let r = ((298 * c + 409 * e + 128) >> 8).clamp(0, 255) as u8;
                let g = ((298 * c - 100 * d - 208 * e + 128) >> 8).clamp(0, 255) as u8;
                let b = ((298 * c + 516 * d + 128) >> 8).clamp(0, 255) as u8;

                rgba_pixel_data.push(r);
                rgba_pixel_data.push(g);
                rgba_pixel_data.push(b);
                rgba_pixel_data.push(255);
            }
        }

        Ok(rgba_pixel_data)
    }

    pub fn process_camera_frame(&mut self) -> Result<RgbaImage, Box<dyn Error>> {
        let image_reader = self.create_image_reader()?;
        let image = self.acquire_image(&image_reader)?;

        let (width, height) = self.get_image_dimensions(&image)?;

        let rgba_data = self.convert_yuv_to_rgba(&image, width, height)?;

        // Create an RgbaImage from the rgba_data
        let mut rgba_image = RgbaImage::new(width as u32, height as u32);

        // Fill the image with the RGBA data
        for (i, chunk) in rgba_data.chunks_exact(4).enumerate() {
            let x = (i % width as usize) as u32;
            let y = (i / width as usize) as u32;
            rgba_image.put_pixel(x, y, Rgba([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }

        self.env.call_method(&image, "close", "()V", &[])?;

        Ok(rgba_image)
    }
}

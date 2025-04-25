use image::{RgbaImage, Rgba};

pub enum CameraViewError {
    AccessDenied,
    FailedToGetFrame
}

// To get camera, Camera::new()
// To get frame, camera.frame()
// impl Drop for Camera and run camera.stop() 

#[derive(Default, Debug)]
pub struct Camera;

impl Camera {
    pub fn new() -> Self {
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        unsafe { start_camera_capture(); }
        Camera
    }

    #[cfg(target_os = "android")]
    pub fn get_frame(&self) -> Result<RgbaImage, CameraViewError> {
        Err(CameraViewError::FailedToGetFrame)
    }

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    pub fn get_frame(&self) -> Result<RgbaImage, CameraViewError> {
        let camera_access_status = unsafe { check_camera_access() };
        if camera_access_status.is_null() {return Err(CameraViewError::FailedToGetFrame)}
        let cstr = unsafe { std::ffi::CStr::from_ptr(camera_access_status) };

        if cstr.to_string_lossy().into_owned().as_str() == "AccessDenied" {
            return Err(CameraViewError::AccessDenied);
        }

        unsafe {
            let ptr = get_latest_frame();
            let size = get_initial_frame_size();
            let stride = get_latest_frame_stride() as usize;
            let width = get_initial_frame_width() as u32;
            let height = get_initial_frame_height() as u32;
    
            if ptr.is_null() || size <= 0 || width == 0 || height == 0 {
                return Err(CameraViewError::FailedToGetFrame);
            }
    
            let slice = std::slice::from_raw_parts(ptr as *const u8, size as usize);
            let mut image = RgbaImage::new(width, height);
    
            let mut pixels = image.pixels_mut();
    
            for y in 0..height {
                let row_start = y as usize * stride;
                for x in 0..width {
                    let src_index = row_start + x as usize * 4;
                    if src_index + 3 >= slice.len() {
                        continue;
                    }
    
                    let r = slice[src_index + 2];
                    let g = slice[src_index + 1]; 
                    let b = slice[src_index];
                    let a = slice[src_index + 3]; 
    
                    let pixel = pixels.next().unwrap();
                    *pixel = Rgba([r, g, b, a]);
                }
            }
    
            #[cfg(target_os = "ios")]
            return Ok(image::imageops::rotate90(&image));
            #[cfg(not(target_os = "ios"))]
            return Ok(image);
        }
    }

    pub fn stop(self) { drop(self); }
}

impl Drop for Camera {
    fn drop(&mut self) {
        println!("Stopping Camera");
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
extern "C" {
    fn start_camera_capture();
    fn check_camera_access() -> *const std::ffi::c_char;
    fn get_latest_frame() -> *mut std::ffi::c_void;
    fn get_latest_frame_stride() -> i32;
    fn get_initial_frame_size() -> i32;
    fn get_initial_frame_width() -> i32;
    fn get_initial_frame_height() -> i32;
}
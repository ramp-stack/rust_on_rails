use image::RgbaImage;
use libc::tolower;
#[cfg(target_os = "android")]
use crate::base::driver::android::camera::*;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use crate::base::driver::apple::camera::*;

#[derive(Debug)]
pub enum CameraError {
    AccessDenied,
    WaitingForAccess,
    FailedToGetFrame
}

#[derive(Debug)]
pub struct Camera (
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    AppleCamera,
    #[cfg(target_os = "android")]
    AndroidCamera,
);

impl Camera {
    pub fn new() -> Self {
        #[cfg(target_os = "ios")]
        start_camera_apple();
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            let camera = AppleCamera::new();
            camera.open_camera();
            return Camera(camera)
        }
       #[cfg(target_os = "android")]
        {
            let mut camera = AndroidCamera::new().expect("Failed to create Android camera");
            camera.open_camera();
            return Camera(camera)
        }
    }

    pub fn get_frame(&mut self) -> Result<RgbaImage, CameraError> {
        #[cfg(target_os = "android")]
        return self.0.get_latest_frame().map_err(|_| CameraError::FailedToGetFrame);

        #[cfg(any(target_os = "ios", target_os = "macos"))]
        return self.0.get_latest_frame().ok_or(CameraError::FailedToGetFrame);

        Err(CameraError::FailedToGetFrame)
    }
}

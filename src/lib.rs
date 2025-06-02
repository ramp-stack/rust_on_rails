#![doc(html_logo_url = "https://raw.githubusercontent.com/ramp-stack/rust_on_rails/main/logo.png")]

mod base;
pub use base::{BackgroundApp, HeadlessContext, BaseApp};
pub use base::window::WindowApp;
pub use base::renderer::RenderApp;
pub use base::driver::runtime::{Task, Tasks, async_trait};
pub use base::driver::state::{State, Field};
pub use base::driver::cache::Cache;
pub use base::driver::app_support::ApplicationSupport;
pub use base::driver::photo_picker::ImageOrientation;
pub use base::driver::cloud::CloudStorage;
pub use base::driver::camera::{Camera, CameraError};
#[cfg(target_os="android")]
pub use winit_crate::platform::android::activity::AndroidApp;

#[cfg(feature = "canvas")]
mod canvas;
#[cfg(feature = "canvas")]
pub mod prelude {
    pub use crate::*;
    pub use crate::canvas::*;
}

#[cfg(feature = "components")]
mod components;
#[cfg(feature = "components")]
pub mod prelude {
    pub use crate::*;
    pub use crate::components::*;
}

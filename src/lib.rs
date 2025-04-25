mod base;
pub use base::{BackgroundApp, HeadlessContext, BaseApp};
pub use base::window::WindowApp;
pub use base::renderer::RenderApp;
pub use base::driver::runtime::{Task, Tasks, async_trait};
pub use base::driver::state::{State, Field};
pub use base::driver::cache::Cache;
pub use base::driver::camera::{Camera, CameraViewError};
#[cfg(target_os="ios")]
pub use base::get_application_support_dir;
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

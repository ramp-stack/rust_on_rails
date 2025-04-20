mod base;
pub use base::{BackgroundApp, HeadlessContext, BaseApp};
pub use base::runtime::{Task, Tasks, async_trait};
pub use base::window::WindowApp;
pub use base::driver::state::{State, Field};

#[cfg(feature = "canvas")]
mod canvas;
#[cfg(feature = "canvas")]
pub mod prelude {
    pub use crate::canvas::*;
    pub use crate::*;
}

#[cfg(feature = "components")]
mod components;
#[cfg(feature = "components")]
pub mod prelude {
    pub use crate::components::*;
    pub use crate::*;
}

mod base;
pub use base::*;

mod canvas;
pub use canvas::CanvasApp;

mod components;

pub mod prelude {
    pub use crate::*;

    pub use canvas::Align as TextAlign;

    pub use components::*;
    pub use components::ComponentAppTrait as App;
    pub use components::ComponentContext as Context;
    pub use crate::create_component_entry_points as create_entry_points;

    pub use include_dir;
    pub use include_dir::include_dir as include_assets;

    pub use base::camera::*;

    pub use proc::{Component, Plugin};
}

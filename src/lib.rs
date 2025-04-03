mod winit;
pub use winit::{WinitAppTrait, WinitApp};

#[cfg(target_os = "android")]
pub use winit::AndroidApp;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;

pub mod canvas;
pub use crate::canvas::CanvasApp;

#[cfg(feature = "canvas")]
pub mod prelude {
    pub use crate::*;
    pub use crate::canvas::{MouseEvent, MouseState, KeyboardEvent, KeyboardState};
    pub use crate::canvas::{CanvasItem, Area, Color, Shape, Text, Image, Font};
    pub use crate::canvas::CanvasAppTrait as App;
    pub use crate::canvas::CanvasContext as Context;
    pub use crate::create_canvas_entry_points as create_entry_points;
}

#[cfg(feature = "components")]
mod components;
#[cfg(feature = "components")]
pub use components::ComponentApp;

#[cfg(feature = "components")]
pub mod prelude {
    pub use crate::*;
    pub use crate::components::{MouseEvent, MouseState, KeyboardEvent, KeyboardState};
    pub use components::{resources, Plugin, Color, Shape, ShapeType, Image, Text, Layout, Component, DefaultLayout, Drawable, Events, SizeInfo};

    pub use crate::components::ComponentAppTrait as App;
    pub use crate::components::ComponentContext as Context;
    pub use crate::create_component_entry_points as create_entry_points;
    pub use include_dir;
    pub use include_dir::include_dir as include_assets;

    pub use proc::{Component, Plugin};
}

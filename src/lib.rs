mod winit;
pub use winit::{WinitAppTrait, WinitApp};

#[cfg(target_os = "android")]
pub use winit::AndroidApp;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;

mod canvas;
pub use canvas::{CanvasAppTrait, CanvasContext, CanvasApp};
pub use canvas::{ItemType, Shape as ShapeType, Text as CanvasText, image, ImageKey, FontKey, CanvasItem, DrawCommand};

#[cfg(feature = "canvas")]
pub mod prelude {
    pub use crate::*;
    pub use crate::CanvasText as Text;
    pub use crate::CanvasAppTrait as App;
    pub use crate::CanvasContext as Context;
    pub use crate::create_canvas_entry_points as create_entry_points;
}

#[cfg(feature = "components")]
mod components;
#[cfg(feature = "components")]
pub use components::{ComponentAppTrait, ComponentContext, ComponentApp, Handle, Image, Shape, Text, ComponentBuilder, Drawable, Vec2, Rect};

#[cfg(feature = "components")]
pub mod prelude {
    pub use crate::*;
    pub use crate::ComponentAppTrait as App;
    pub use crate::ComponentContext as Context;
    pub use crate::create_component_entry_points as create_entry_points;
    pub use crate::{Drawable, Vec2};
    pub use include_dir;
    pub use include_dir::include_dir as include_assets;
}

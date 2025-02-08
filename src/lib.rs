mod winit;
pub use winit::{WinitAppTrait, WinitApp};
use winit::WinitWindow;

mod canvas;
pub use canvas::{CanvasAppTrait, CanvasApp, CanvasAtlas};
pub use canvas::{ItemType, Shape, CanvasItem, Text, image};

//  mod components;
//  pub use components::{ComponentAppTrait, ComponentApp};
//  pub use components::{ComponentContext, *};

//  pub mod prelude {
//      #[cfg(target_os = "android")]
//      pub use winit::AndroidApp;

//      #[cfg(target_arch = "wasm32")]
//      pub use wasm_bindgen::prelude::*;

//      #[cfg(target_arch = "wasm32")]
//      pub use wasm_bindgen;

//      pub use crate::{WinitApp, WinitAppTrait, CanvasApp, CanvasAppTrait};//, CanvasContext, ComponentApp};
//      pub use crate::{
//          create_winit_entry_points,
//          create_canvas_entry_points,
//          //create_entry_points
//      };
//    //pub use crate::ComponentAppTrait as App;
//    //pub use crate::ComponentContext as Context;
//    //pub use crate::*;
//  }

mod winit;
pub use winit::{WinitWindow, ScreenSize, WinitApp, Winit};

#[cfg(target_os = "android")]
pub use winit::AndroidApp;

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;

mod wgpu;
pub use wgpu::{CanvasApp, MeshType, Context, Shape, Mesh, App};

pub type LogLevel = log::Level;

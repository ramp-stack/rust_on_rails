use std::future::Future;
use raw_window_handle::{HasWindowHandle, HasDisplayHandle};

///WindowHandle provides a trait for any generic Window that the Renderers can use
///Alias for raw_window_handle traits
pub trait WindowHandle: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static {}
impl<W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static> WindowHandle for W {}

//TODO: Replace with non winit structs
pub use winit_crate::keyboard::{NamedKey, SmolStr, Key};

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent<W: WindowHandle> {
    Resized{width: u32, height: u32, scale_factor: f64},
    Mouse{position: (u32, u32), state: MouseState},
    Keyboard{key: Key, state: KeyboardState},
    Resumed{window: W, width: u32, height: u32, scale_factor: f64},
    Paused,
    Tick
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseState{ Pressed, Moved, Released }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardState{ Pressed, Released }

pub trait WindowAppTrait {
    fn new<W: WindowHandle>(
        name: &str, window: W, width: u32, height: u32, scale_factor: f64
    ) -> impl Future<Output = Self> where Self: Sized;
    fn on_event<W: WindowHandle>(&mut self, event: WindowEvent<W>) -> impl Future<Output = ()>;
    fn close(self) -> impl Future<Output = ()>;
}

mod winit;
pub use winit::Winit as WindowApp;

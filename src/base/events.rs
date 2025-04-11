//TODO: Replace winit event data with custom event data
pub use winit_crate::keyboard::{NamedKey, SmolStr, Key};

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Resize{width: u32, height: u32, scale_factor: f64},
    Mouse{position: (u32, u32), state: MouseState},
    Keyboard{key: Key, state: KeyboardState},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseState {
    Pressed,
    Moved,
    Released
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardState {
    Pressed,
    Released
}

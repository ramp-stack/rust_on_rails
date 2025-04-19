#[cfg(target_os="linux")]
mod linux;
#[cfg(target_os="linux")]
pub use linux::{_BackgroundApp, BaseApp};

#[cfg(target_os="macos")]
mod macos;
#[cfg(target_os="macos")]
pub use macos::{_BackgroundApp, BaseApp};

#[cfg(target_os="ios")]
mod ios;
#[cfg(target_os="ios")]
pub use ios::{_BackgroundApp, BaseApp};

#[cfg(target_os="android")]
mod android;
#[cfg(target_os="android")]
pub use android::{_BackgroundApp, BaseApp, AndroidApp};

use crate::base;
use crate::base::{MouseState, KeyboardState};
use winit_crate::event::{ElementState, WindowEvent, TouchPhase, Touch};

#[derive(Default)]
pub struct WinitEventHandler {
    scale_factor: f64,
    mouse: (u32, u32),
    size: (u32, u32),
}

impl WinitEventHandler {
    fn convert_event(&mut self, event: WindowEvent) -> Option<base::WindowEvent> {
        match event {
            WindowEvent::Resized(size) => {
                self.size = (size.width, size.height);
                Some(base::WindowEvent::Resize{
                    width: size.width, height: size.height, scale_factor: self.scale_factor
                })
            },
            WindowEvent::ScaleFactorChanged{scale_factor, ..} => {
                self.scale_factor = scale_factor;
                Some(base::WindowEvent::Resize{
                    width: self.size.0, height: self.size.1, scale_factor
                })
            },
            WindowEvent::Touch(Touch{location, phase, ..}) => {
                self.mouse = (location.x as u32, location.y as u32);
                Some(base::WindowEvent::Mouse{position: self.mouse, state: match phase {
                    TouchPhase::Started => MouseState::Pressed,
                    TouchPhase::Moved => MouseState::Moved,
                    TouchPhase::Ended => MouseState::Released,
                    TouchPhase::Cancelled => MouseState::Released
                }})
            },
            WindowEvent::CursorMoved{position, ..} => {
                if self.mouse != (position.x as u32, position.y as u32) {
                    self.mouse = (position.x as u32, position.y as u32);
                    Some(base::WindowEvent::Mouse{position: self.mouse, state: MouseState::Moved})
                } else {None}
            },
            WindowEvent::MouseInput{state, ..} => {
                Some(base::WindowEvent::Mouse{position: self.mouse, state: match state {
                    ElementState::Pressed => MouseState::Pressed,
                    ElementState::Released => MouseState::Released,
                }})
            },
            WindowEvent::KeyboardInput{event, ..} => {
                Some(base::WindowEvent::Keyboard{key: event.logical_key, state: match event.state {
                    ElementState::Pressed => KeyboardState::Pressed,
                    ElementState::Released => KeyboardState::Released,
                }})
            },
            _ => None
        }
    }
}

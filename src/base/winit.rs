use winit_crate::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit_crate::event::{ElementState, WindowEvent, TouchPhase, Touch};
use winit_crate::application::ApplicationHandler;
use winit_crate::window::{Window, WindowId};

#[cfg(target_os = "android")]
use winit_crate::platform::android::EventLoopBuilderExtAndroid;
#[cfg(target_os = "android")]
pub use winit_crate::platform::android::activity::AndroidApp;

#[cfg(target_arch="wasm32")]
use winit_crate::platform::web::{WindowExtWebSys, EventLoopExtWebSys};

use std::sync::Arc;
use crate::base::*;

pub struct WinitApp<A: BaseAppTrait + 'static> {
    window: Option<Arc<Window>>,
    mouse: (u32, u32),
    app: Option<BaseApp<A>>,
}

impl<A: BaseAppTrait + 'static> WinitApp<A> {
    pub fn new(app: BaseApp<A>) -> Self {
        WinitApp{
            window: None,
            mouse: (0, 0),
            app: Some(app)
        }
    }

    #[cfg(target_os="android")]
    pub fn start(&mut self, app: AndroidApp) {
        let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).unwrap();
    }
    #[cfg(target_arch="wasm32")]
    pub fn start(self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.spawn_app(self);
    }
    #[cfg(not(any(target_os="android", target_arch="wasm32")))]
    pub fn start(&mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).unwrap();
    }

    fn window(&self) -> Arc<Window> {self.window.clone().unwrap()}
    fn app(&mut self) -> &mut BaseApp<A> {self.app.as_mut().unwrap()}
}

impl<A: BaseAppTrait + 'static> ApplicationHandler for WinitApp<A> {
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window().request_redraw();
     }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
         self.window = Some(Arc::new(event_loop.create_window(
            Window::default_attributes().with_title("orange")
        ).unwrap()));
        #[cfg(target_arch="wasm32")]
        {
            web_sys::window().and_then(|win| win.document()).and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(self.window().canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            }).expect("Couldn't append canvas to document body.");
            let _ = self.window().request_inner_size(winit_crate::dpi::PhysicalSize::new(450, 400));
        }
        let size = self.window().inner_size();
        let scale_factor = self.window().scale_factor();
        let window = self.window.clone().unwrap();
        self.app().on_resume(window, size.width, size.height, scale_factor);
        self.window().request_redraw();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.app().on_pause();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if id == self.window().id() {
            match event {
                WindowEvent::CloseRequested => {
                    self.app.take().unwrap().on_close();
                    event_loop.exit();
                },
                WindowEvent::RedrawRequested => {
                    if self.app.is_some() {//Winit tries for another tick after close
                        self.app().on_tick();
                        self.window().request_redraw();
                    }
                },
                WindowEvent::Occluded(occluded) => {
                    if occluded {self.app().on_pause();} else {
                        let window = self.window.clone().unwrap();
                        let size = self.window().inner_size();
                        let scale_factor = self.window().scale_factor();
                        self.app().on_resume(window, size.width, size.height, scale_factor);
                    }
                },
                WindowEvent::Resized(size) => {
                    let scale_factor = self.window().scale_factor();
                    self.app().on_event(
                        Event::Resize{width: size.width, height: size.height, scale_factor}
                    );
                    self.window().request_redraw();
                },
                WindowEvent::ScaleFactorChanged{scale_factor, ..} => {
                    let size = self.window().inner_size();
                    self.app().on_event(
                        Event::Resize{width: size.width, height: size.height, scale_factor}
                    );
                    self.window().request_redraw();
                },
                WindowEvent::Touch(Touch{location, phase, ..}) => {
                    self.mouse = (location.x as u32, location.y as u32);
                    self.app.as_mut().unwrap().on_event(
                        Event::Mouse{position: self.mouse, state: match phase {
                        TouchPhase::Started => MouseState::Pressed,
                        TouchPhase::Moved => MouseState::Moved,
                        TouchPhase::Ended => MouseState::Released,
                        TouchPhase::Cancelled => MouseState::Released
                    }});
                },
                WindowEvent::CursorMoved{position, ..} => {
                    if self.mouse != (position.x as u32, position.y as u32) {
                        self.mouse = (position.x as u32, position.y as u32);
                        self.app.as_mut().unwrap().on_event(
                            Event::Mouse{position: self.mouse, state: MouseState::Moved}
                        );
                    }
                },
                WindowEvent::MouseInput{state, ..} => {
                    self.app.as_mut().unwrap().on_event(
                        Event::Mouse{position: self.mouse, state: match state {
                            ElementState::Pressed => MouseState::Pressed,
                            ElementState::Released => MouseState::Released,
                        }}
                    );
                },
                WindowEvent::KeyboardInput{event, ..} => {
                    self.app.as_mut().unwrap().on_event(
                        Event::Keyboard{key: event.logical_key, state: match event.state {
                            ElementState::Pressed => KeyboardState::Pressed,
                            ElementState::Released => KeyboardState::Released,
                        }}
                    );
                },
                _ => {}
            }
        }
    }
}

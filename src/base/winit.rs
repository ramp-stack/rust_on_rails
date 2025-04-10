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
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        WinitApp{
            window: None,
            mouse: (0, 0),
            app: None
        }
    }

    #[cfg(target_os="android")]
    pub fn start_logger() {
        android_logger::init_once(
            android_logger::Config::default().with_max_level(A::LOG_LEVEL.to_level_filter()),
        );
    }
    #[cfg(target_arch="wasm32")]
    pub fn start_logger() {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(A::LOG_LEVEL).expect("Couldn't initialize logger");
    }
    #[cfg(not(target_arch="wasm32"))]
    #[cfg(not(target_os="android"))]
    pub fn start_logger() {
        env_logger::builder().filter_level(A::LOG_LEVEL.to_level_filter()).init();
    }


    #[cfg(target_os="android")]
    pub fn start(&mut self, app: AndroidApp) {
        Self::start_logger();
        let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).unwrap();
    }
    #[cfg(target_arch="wasm32")]
    pub fn start(self) {
        Self::start_logger(A::LOG_LEVEL);
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.spawn_app(self);
    }
    #[cfg(not(any(target_os="android", target_arch="wasm32")))]
    pub fn start(&mut self) {
        Self::start_logger();
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).unwrap();
    }

    #[cfg(target_os="ios")]
    pub fn background() {
        Self::start_logger();
        A::on_background_tick();
    }

    fn window(&self) -> Arc<Window> {self.window.clone().unwrap()}
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

        if self.app.is_none() {
            self.app = Some(BaseApp::new(self.window(), size.width, size.height, scale_factor));
        } else {
            self.app.as_mut().unwrap().on_resume(self.window.clone().unwrap());
            self.app.as_mut().unwrap().on_resize(size.width, size.height, scale_factor);
        }
        self.window().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if id == self.window().id() {
            match event {
                WindowEvent::CloseRequested => {
                    self.app.take().unwrap().on_close();
                    event_loop.exit();
                },
                WindowEvent::RedrawRequested => {
                    self.app.as_mut().unwrap().on_tick();
                    self.window().request_redraw();
                },
                WindowEvent::Resized(size) => {
                    let scale_factor = self.window().scale_factor();
                    self.app.as_mut().unwrap().on_resize(size.width, size.height, scale_factor);
                    self.window().request_redraw();
                },
                WindowEvent::ScaleFactorChanged{scale_factor, ..} => {
                    let size = self.window().inner_size();
                    self.app.as_mut().unwrap().on_resize(size.width, size.height, scale_factor);
                    self.window().request_redraw();
                },
                WindowEvent::Touch(Touch{location, phase, ..}) => {
                    self.mouse = (location.x as u32, location.y as u32);
                    self.app.as_mut().unwrap().on_mouse(MouseEvent{position: self.mouse, state: match phase {
                        TouchPhase::Started => MouseState::Pressed,
                        TouchPhase::Moved => MouseState::Moved,
                        TouchPhase::Ended => MouseState::Released,
                        TouchPhase::Cancelled => MouseState::Released
                    }})
                },
                WindowEvent::CursorMoved{position, ..} => {
                    if self.mouse != (position.x as u32, position.y as u32) {
                        self.mouse = (position.x as u32, position.y as u32);
                        self.app.as_mut().unwrap().on_mouse(MouseEvent{position: self.mouse, state: MouseState::Moved});
                    }
                },
                WindowEvent::MouseInput{state, ..} => {
                    self.app.as_mut().unwrap().on_mouse(MouseEvent{position: self.mouse, state: match state {
                        ElementState::Pressed => MouseState::Pressed,
                        ElementState::Released => MouseState::Released,
                    }})
                },
                WindowEvent::KeyboardInput{event, ..} => {
                    self.app.as_mut().unwrap().on_keyboard(KeyboardEvent{key: event.logical_key, state: match event.state {
                        ElementState::Pressed => KeyboardState::Pressed,
                        ElementState::Released => KeyboardState::Released,
                    }})
                },
                _ => {}
            }
        }
    }
}

#[macro_export]
macro_rules! create_winit_entry_points {
    ($app:ty) => {
        #[cfg(target_arch = "wasm32")]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
        pub fn wasm_main() {
            WinitApp::<$app>::new().start()
        }

        #[cfg(target_os = "android")]
        #[no_mangle]
        fn android_main(app: AndroidApp) {
            WinitApp::<$app>::new().start(app)
        }

        #[cfg(target_os = "ios")]
        #[no_mangle]
        pub extern "C" fn ios_main() {
            WinitApp::<$app>::new().start()
        }

        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        pub fn desktop_main() {
            WinitApp::<$app>::new().start()
        }

        #[cfg(target_os = "ios")]
        #[no_mangle]
        pub extern "C" fn ios_background() {
            WinitApp::<$app>::background()
        }
    };
}

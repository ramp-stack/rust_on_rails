use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::event::{ElementState, WindowEvent, KeyEvent, TouchPhase, Touch};
use winit::keyboard::{PhysicalKey, KeyCode};
use winit::application::ApplicationHandler;
use winit::window::{Window, WindowId};

#[cfg(target_os = "android")]
use winit::platform::android::EventLoopBuilderExtAndroid;
#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

#[cfg(target_arch="wasm32")]
use winit::platform::web::{WindowExtWebSys, EventLoopExtWebSys};
#[cfg(target_arch="wasm32")]
use std::sync::Mutex;

use std::sync::Arc;

pub use winit::keyboard::{NamedKey, Key};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseState {
    Pressed,
    Moved,
    Released
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub position: (u32, u32),
    pub state: MouseState
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardState {
    Pressed,
    Released
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardEvent {
    pub key: Key,
    pub state: KeyboardState
}


pub type WinitWindow = Arc<Window>;

pub trait WinitAppTrait {
    const LOG_LEVEL: log::Level = log::Level::Error;

    fn new(window: WinitWindow, width: u32, height: u32, scale_factor: f64) -> impl std::future::Future<Output = Self> where Self: Sized;
    fn on_resize(&mut self, width: u32, height: u32, scale_factor: f64);
    fn prepare(&mut self);
    fn render(&mut self);

    fn on_mouse(&mut self, event: MouseEvent);
    fn on_keyboard(&mut self, event: KeyboardEvent);
}

pub struct WinitApp<A: WinitAppTrait> {
    width: u32,
    height: u32,
    scale_factor: f64,
    window: Option<Arc<Window>>,
    app: Option<A>,
    #[cfg(target_arch="wasm32")]
    inbox: Option<Arc<Mutex<Option<A>>>>,
    #[cfg(not(target_arch="wasm32"))]
    runtime: tokio::runtime::Runtime,
    mouse: (u32, u32),
}

impl<A: WinitAppTrait + 'static> WinitApp<A> {
    pub fn new() -> Self {
        WinitApp{
            width: 0,
            height: 0,
            scale_factor: 1.0,
            window: None,
            app: None,
            #[cfg(target_arch="wasm32")]
            inbox: Some(Arc::new(Mutex::new(None))),
            #[cfg(not(target_arch="wasm32"))]
            runtime: tokio::runtime::Runtime::new().unwrap(),
            mouse: (0, 0),
        }
    }

    #[cfg(target_os="android")]
    pub fn start(&mut self, log_level: log::LevelFilter, app: AndroidApp) {
        android_logger::init_once(
            android_logger::Config::default().with_max_level(log_level),
        );

        let event_loop = EventLoop::builder()
        .with_android_app(app)
        .build()
        .unwrap();

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop.run_app(self).unwrap();
    }

    #[cfg(target_arch="wasm32")]
    pub fn start(self, log_level: log::Level) {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log_level).expect("Couldn't initialize logger");

        let event_loop = EventLoop::new().unwrap();

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop.spawn_app(self);
    }

    #[cfg(not(target_arch="wasm32"))]
    #[cfg(not(target_os="android"))]
    pub fn start(&mut self, log_level: log::LevelFilter) {
        env_logger::builder().filter_level(log_level).init();

        let event_loop = EventLoop::new().unwrap();

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop.run_app(self).unwrap();
    }

    fn window(&self) -> Arc<Window> {self.window.clone().unwrap()}
    //fn app(&self) -> Arc<Window> {self.window.clone().unwrap()}
}

impl<A: WinitAppTrait + 'static> ApplicationHandler for WinitApp<A> {
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window().request_redraw();
     }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(Arc::new(event_loop.create_window(Window::default_attributes()).unwrap()));
        let window = self.window().clone();

        let size = window.inner_size();
        self.width = size.width;
        self.height = size.height;
        let scale_factor = window.scale_factor();
        self.scale_factor = scale_factor;


        #[cfg(not(target_arch="wasm32"))]
        {
            self.app = Some(self.runtime.block_on(A::new(self.window(), self.width, self.height, self.scale_factor)));
        }

        #[cfg(target_arch="wasm32")]
        {
            let inbox = self.inbox.as_ref().unwrap().clone();
            web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
            let _ = self.window().request_inner_size(winit::dpi::PhysicalSize::new(450, 400));

            wasm_bindgen_futures::spawn_local(async move {
                *inbox.lock().unwrap() = Some(A::new(window, size.width, size.height, scale_factor).await);
            });
        }

        self.window().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        #[cfg(target_arch="wasm32")]
        {
            if let Some(app) = self.inbox.as_mut() {
                self.app = app.lock().unwrap().take();
            }
        }
        if id == self.window().id() {
            match event {
                WindowEvent::CloseRequested |
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => {
                    println!("The close button was pressed; stopping");
                    event_loop.exit();
                },
                WindowEvent::RedrawRequested => {
                    self.app.as_mut().unwrap().prepare();
                    self.app.as_mut().unwrap().render();
                    self.window().request_redraw();
                },
                WindowEvent::Resized(size) => {
                    self.width = size.width;
                    self.height = size.height;
                    self.app.as_mut().unwrap().on_resize(self.width, self.height, self.scale_factor);
                    self.window().request_redraw();
                },
                WindowEvent::ScaleFactorChanged{scale_factor, ..} => {
                    self.scale_factor = scale_factor;
                    self.app.as_mut().unwrap().on_resize(self.width, self.height, self.scale_factor);
                    self.window().request_redraw();
                },
                WindowEvent::Touch(Touch{location, phase, ..}) => {
                    self.mouse = (location.x as u32, location.y as u32);
                    let state = match phase {
                        TouchPhase::Started => MouseState::Pressed,
                        TouchPhase::Moved => MouseState::Moved,
                        TouchPhase::Ended => MouseState::Released,
                        TouchPhase::Cancelled => MouseState::Released
                    };
                    let event = MouseEvent{position: self.mouse, state};
                    self.app.as_mut().unwrap().on_mouse(event);
                },
                WindowEvent::CursorMoved{position, ..} => {
                    if self.mouse != (position.x as u32, position.y as u32) {
                        self.mouse = (position.x as u32, position.y as u32);
                        let event = MouseEvent{position: self.mouse, state: MouseState::Moved};
                        self.app.as_mut().unwrap().on_mouse(event);
                    }
                },
                WindowEvent::MouseInput{state, ..} => {
                    let state = match state {
                        ElementState::Pressed => MouseState::Pressed,
                        ElementState::Released => MouseState::Released,
                    };
                    let event = MouseEvent{position: self.mouse, state};
                    self.app.as_mut().unwrap().on_mouse(event);
                },
                WindowEvent::KeyboardInput{event, ..} => {
                    let state = match event.state {
                        ElementState::Pressed => KeyboardState::Pressed,
                        ElementState::Released => KeyboardState::Released,
                    };
                    let event = KeyboardEvent{key: event.logical_key, state};
                    self.app.as_mut().unwrap().on_keyboard(event);
                },
                _ => {}
            }
        }
    }
}

impl<A: WinitAppTrait + 'static> Default for WinitApp<A> {
    fn default() -> Self {
        Self::new()
    }
}

#[macro_export]
macro_rules! create_winit_entry_points {
    ($app:ty) => {
        #[cfg(target_os = "android")]
        #[no_mangle]
        fn android_main(app: AndroidApp) {
            WinitApp::<$app>::new().start(<$app>::LOG_LEVEL.to_level_filter(), app)
        }

        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        pub fn desktop_main() {
            WinitApp::<$app>::new().start(<$app>::LOG_LEVEL.to_level_filter())
        }

        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        #[no_mangle]
        pub extern "C" fn ios_main() {
            WinitApp::<$app>::new().start(<$app>::LOG_LEVEL.to_level_filter())
        }

        #[cfg(target_arch = "wasm32")]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
        pub fn wasm_main() {
            WinitApp::<$app>::new().start(<$app>::LOG_LEVEL)
        }
    };
}

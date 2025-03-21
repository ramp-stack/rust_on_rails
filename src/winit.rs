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

use std::sync::{Mutex, Arc};

pub type WinitWindow = Arc<Window>;

pub trait WinitAppTrait {
    const LOG_LEVEL: log::Level = log::Level::Error;

    fn new(window: WinitWindow, width: u32, height: u32, scale_factor: f64) -> impl std::future::Future<Output = Self> where Self: Sized;
    fn prepare(&mut self, width: u32, height: u32, scale_factor: f64) -> impl std::future::Future<Output = ()>;
    fn render(&mut self) -> impl std::future::Future<Output = ()>;

    fn on_click(&mut self) -> impl std::future::Future<Output = ()>;
    fn on_move(&mut self, x: u32, y: u32) -> impl std::future::Future<Output = ()>;
    fn on_press(&mut self, t: String) -> impl std::future::Future<Output = ()>;
}

pub struct WinitApp<A: WinitAppTrait> {
    width: u32,
    height: u32,
    scale_factor: f64,
    window: Option<Arc<Window>>,
    app: Arc<Mutex<Option<A>>>,
    #[cfg(not(target_arch="wasm32"))]
    runtime: tokio::runtime::Runtime,
    mouse: (u32, u32)
}

impl<A: WinitAppTrait + 'static> WinitApp<A> {
    pub fn new() -> Self {
        WinitApp{
            width: 0,
            height: 0,
            scale_factor: 1.0,
            window: None,
            app: Arc::new(Mutex::new(None)),
            #[cfg(not(target_arch="wasm32"))]
            runtime: tokio::runtime::Runtime::new().unwrap(),
            mouse: (0, 0)
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
        self.scale_factor = window.scale_factor();

        let app = self.app.clone();

        #[cfg(not(target_arch="wasm32"))]
        {
            *app.lock().unwrap() = Some(self.runtime.block_on(A::new(self.window(), self.width, self.height, self.scale_factor)));
        }

        #[cfg(target_arch="wasm32")]
        {
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
                *app.lock().unwrap() = Some(A::new(window).await);
            });
        }

        self.window().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
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
                    let width = self.width;
                    let height = self.height;
                    let scale_factor = self.scale_factor;

                    #[cfg(target_arch="wasm32")]
                    {
                        let app = self.app.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let mut app = app.lock().unwrap();
                            app.as_mut().unwrap().prepare(
                                width, height, scale_factor
                            ).await;

                            app.as_mut().unwrap().render().await;
                            drop(app);
                        });
                    }

                    #[cfg(not(target_arch="wasm32"))]
                    {
                        self.runtime.block_on(
                            self.app.lock().unwrap().as_mut().unwrap().prepare(
                                width, height, scale_factor
                            )
                        );
                        self.runtime.block_on(
                            self.app.lock().unwrap().as_mut().unwrap().render()
                        );
                    }

                    self.window().request_redraw();
                },
                WindowEvent::Resized(size) => {
                    self.width = size.width;
                    self.height = size.height;
                    self.window().request_redraw();
                },
                WindowEvent::ScaleFactorChanged{scale_factor, ..} => {
                    self.scale_factor = scale_factor;
                    self.window().request_redraw();
                },
                WindowEvent::Touch(Touch{location, phase: TouchPhase::Started, ..}) => {
                    self.mouse = (location.x as u32, location.y as u32);
                    let app = self.app.clone();
                    #[cfg(not(target_arch="wasm32"))]
                    {
                        self.runtime.block_on(app.lock().unwrap().as_mut().unwrap().on_click());
                    }

                    #[cfg(target_arch="wasm32")]
                    {
                        wasm_bindgen_futures::spawn_local(async move {
                            app.lock().unwrap().as_mut().unwrap().on_click().await;
                        });
                    }
                }
                WindowEvent::Touch(Touch{location, phase: TouchPhase::Moved, ..}) => {
                    if self.mouse != (location.x as u32, location.y as u32) {
                        self.mouse = (location.x as u32, location.y as u32);
                        let app = self.app.clone();
                        #[cfg(not(target_arch="wasm32"))]
                        {
                            self.runtime.block_on(app.lock().unwrap().as_mut().unwrap().on_move(location.x as u32, location.y as u32));
                        }

                        #[cfg(target_arch="wasm32")]
                        {
                            wasm_bindgen_futures::spawn_local(async move {
                                app.lock().unwrap().as_mut().unwrap().on_move(location.x as u32, location.y as u32).await;
                            });
                        }
                    }
                },
                WindowEvent::CursorMoved{position, ..} => {
                    if self.mouse != (position.x as u32, position.y as u32) {
                        self.mouse = (position.x as u32, position.y as u32);
                        let app = self.app.clone();
                        #[cfg(not(target_arch="wasm32"))]
                        {
                            self.runtime.block_on(app.lock().unwrap().as_mut().unwrap().on_move(position.x as u32, position.y as u32));
                        }

                        #[cfg(target_arch="wasm32")]
                        {
                            wasm_bindgen_futures::spawn_local(async move {
                                app.lock().unwrap().as_mut().unwrap().on_move(position.x as u32, position.y as u32).await;
                            });
                        }
                    }
                },
                WindowEvent::MouseInput{state: ElementState::Pressed, ..} => {
                    let app = self.app.clone();
                    #[cfg(not(target_arch="wasm32"))]
                    {
                        self.runtime.block_on(app.lock().unwrap().as_mut().unwrap().on_click());
                    }

                    #[cfg(target_arch="wasm32")]
                    {
                        wasm_bindgen_futures::spawn_local(async move {
                            app.lock().unwrap().as_mut().unwrap().on_click().await;
                        });
                    }
                },
                WindowEvent::KeyboardInput{event: KeyEvent{state: ElementState::Pressed, text: Some(text), ..}, ..} => {
                    let app = self.app.clone();
                    #[cfg(not(target_arch="wasm32"))]
                    {
                        self.runtime.block_on(app.lock().unwrap().as_mut().unwrap().on_press(text.to_string()));
                    }

                    #[cfg(target_arch="wasm32")]
                    {
                        wasm_bindgen_futures::spawn_local(async move {
                            app.lock().unwrap().as_mut().unwrap().on_press(text.to_string()).await;
                        });
                    }
                },
                _ => {
                    //log::error!("{:?}", event);
                },
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

use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::event::{ElementState, WindowEvent, KeyEvent};
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

//  #[derive(Clone, Copy)]
//  pub struct ScreenSize {
//      pub physical_width: u32,
//      pub physical_height: u32,
//      pub scale_factor: f64,
//      pub logical_width: f32,
//      pub logical_height: f32
//  }

//  impl ScreenSize {
//      pub fn new(physical_width: u32, physical_height: u32, scale_factor: f64) -> Self {
//          ScreenSize{
//              physical_width, physical_height, scale_factor,
//              logical_width: (physical_width as f64 * scale_factor) as f32,
//              logical_height: (physical_height as f64 * scale_factor) as f32,
//          }
//      }
//  }

pub type WinitWindow = Arc<Window>;

pub trait WinitAppTrait {
    const LOG_LEVEL: log::Level = log::Level::Info;

    fn new(window: WinitWindow) -> impl std::future::Future<Output = Self> where Self: Sized;
    fn prepare(&mut self, width: u32, height: u32, scale_factor: f64, logical_width: f32, logical_height: f32) -> impl std::future::Future<Output = ()>;
    fn render(&mut self) -> impl std::future::Future<Output = ()>;
}

pub struct WinitApp<A: WinitAppTrait> {
    width: u32,
    height: u32,
    scale_factor: f64,
    window: Option<Arc<Window>>,
    app: Arc<Mutex<Option<A>>>,
    #[cfg(not(target_arch="wasm32"))]
    runtime: tokio::runtime::Runtime
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
            runtime: tokio::runtime::Runtime::new().unwrap()
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
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(Arc::new(event_loop.create_window(Window::default_attributes()).unwrap()));

        let size = self.window().inner_size();
        self.width = size.width;
        self.height = size.height;
        self.scale_factor = self.window().scale_factor();

        #[cfg(not(target_arch="wasm32"))]
        {
            *self.app.lock().unwrap() = Some(self.runtime.block_on(A::new(self.window())));
        }

        #[cfg(target_arch="wasm32")]
        {
            let window = self.window().clone();
            web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(self.window().canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");

            let _ = self.window().request_inner_size(winit::dpi::PhysicalSize::new(450, 400));

            let app = self.app.clone();

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
                    let logical_width = (width as f64 * scale_factor) as f32;
                    let logical_height = (height as f64 * scale_factor) as f32;


                    #[cfg(target_arch="wasm32")]
                    {
                        let app = self.app.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let mut app = app.lock().unwrap();
                            app.as_mut().unwrap().prepare(
                                width, height, scale_factor, logical_width, logical_height
                            ).await;

                            app.as_mut().unwrap().render().await;
                            drop(app);
                        });
                    }

                    #[cfg(not(target_arch="wasm32"))]
                    {
                        self.runtime.block_on(
                            self.app.lock().unwrap().as_mut().unwrap().prepare(
                                width, height, scale_factor, logical_width, logical_height
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
                }
                _ => (),
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

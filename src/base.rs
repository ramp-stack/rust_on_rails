use std::future::Future;
use std::path::PathBuf;

pub mod driver;
use driver::logger::Logger;
use driver::state::State;
use driver::cache::Cache;
use driver::camera::Camera;

pub mod runtime;
use runtime::{BlockingRuntime, Runtime, Tasks};

pub mod window;
use window::{WindowAppTrait, WindowHandle, WindowEvent};
pub use window::{MouseState, KeyboardState, NamedKey, SmolStr, Key};

pub mod renderer;
pub use renderer::Renderer;

pub trait BaseAppTrait<R: Renderer> {
    const LOG_LEVEL: log::Level;
    fn background_tasks(ctx: &mut HeadlessContext) -> impl Future<Output = Tasks> where Self: Sized;
    fn new(
        ctx: &mut Context<R>, h_ctx: &mut HeadlessContext, width: f32, height: f32
    ) -> impl Future<Output = (Self, Tasks)> where Self: Sized;
    fn on_event(&mut self, ctx: &mut Context<R>, event: Event);
    fn draw(&mut self, ctx: &mut Context<R>) -> R::Input;
    fn close(self) -> impl Future<Output = ()>;
}

///Event provides access to all window events in logical pixels
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Resized{width: f32, height: f32},
    Mouse{position: (f32, f32), state: MouseState},
    Keyboard{key: Key, state: KeyboardState},
    Resumed{width: f32, height: f32},
    Paused,
    Tick
}

pub struct HeadlessContext {
    pub cache: Cache,
}

impl HeadlessContext {
    async fn new(storage_path: PathBuf) -> Self {
        HeadlessContext{
            cache: Cache::new(storage_path).await,
        }
    }
}

pub struct Context<R: Renderer> {
    pub cache: Cache,
    pub state: State,
    pub r_ctx: R::Context
}

impl<R: Renderer> AsMut<R::Context> for Context<R> {
    fn as_mut(&mut self) -> &mut R::Context {
        &mut self.r_ctx
    }
}

impl<R: Renderer> Context<R> {
    async fn new(storage_path: PathBuf, r_ctx: R::Context) -> Self {
        Context{
            cache: Cache::new(storage_path).await,
            state: State::default(),
            r_ctx
        }
    }

    pub fn render_ctx(&mut self) -> &mut R::Context {self.as_mut()}

    pub fn state(&mut self) -> &mut State {&mut self.state}
  //TODO: pub fn open_camera(...)
}

pub struct BackgroundApp;
impl BackgroundApp {
    pub fn new_start<R: Renderer, A: BaseAppTrait<R>>(storage_path: PathBuf) {
        let (ctx, tasks) = BlockingRuntime::block_on(async {
            let mut ctx = HeadlessContext::new(storage_path).await;
            let tasks = A::background_tasks(&mut ctx).await;
            (ctx, tasks)
        }).unwrap();
        Runtime::new_background(ctx, tasks);
    }
}

///BaseApp is the heart of rust_on_rails providing all
///of the hardware interfaces for higher level applications(canvas, components)
pub struct BaseApp<R: Renderer, A: BaseAppTrait<R>> {
    runtime: Runtime,
    renderer: R,
    context: Context<R>,
    app: A
}

impl<R: Renderer, A: BaseAppTrait<R>> WindowAppTrait for BaseApp<R, A> {
    async fn new<W: WindowHandle>(
        storage_path: PathBuf, window: W, width: u32, height: u32, scale_factor: f64
    ) -> Self {
        Logger::start(A::LOG_LEVEL);        
        let (renderer, r_ctx, (width, height)) = R::new(window, width, height, scale_factor).await;
        let mut context = Context::new(storage_path.clone(), r_ctx).await;
        let mut headless_ctx = HeadlessContext::new(storage_path).await;
        let (app, tasks) = A::new(&mut context, &mut headless_ctx, width, height).await;
        let runtime = Runtime::new::<R, A>(headless_ctx, tasks);
        BaseApp{renderer, runtime, context, app}
    }

    async fn on_event<W: WindowHandle>(&mut self, event: WindowEvent<W>) {
        let event = match event {
            WindowEvent::Resized{width, height, scale_factor} => {
                let (width, height) = self.renderer.resize::<W>(
                    &mut self.context.r_ctx, None, width, height, scale_factor
                ).await;
                Some(Event::Resized{width, height})
            },
            WindowEvent::Mouse{position, state} => {
                let scale = self.renderer.get_scale(&self.context.r_ctx);
                Some(Event::Mouse{position: (
                    scale.logical(position.0 as f32), scale.logical(position.1 as f32)
                ), state})
            }
            WindowEvent::Keyboard{key, state} => Some(Event::Keyboard{key, state}),
            WindowEvent::Resumed{window, width, height, scale_factor} => {
                self.runtime.resume();
                let (width, height) = self.renderer.resize(
                    &mut self.context.r_ctx, Some(window.into()), width, height, scale_factor
                ).await;
                Some(Event::Resumed{width, height})
            },
            WindowEvent::Paused => {
                self.runtime.pause();
                Some(Event::Paused)
            },
            WindowEvent::Tick => {
                self.app.on_event(&mut self.context, Event::Tick);
                let input = self.app.draw(&mut self.context);
                self.renderer.draw(&mut self.context.r_ctx, input).await;
                None
            }  
        };
        if let Some(event) = event {self.app.on_event(&mut self.context, event);}
    }

    async fn close(mut self) {
        self.runtime.close();
        self.app.close().await;
    }
}

#[macro_export]
macro_rules! create_base_entry_points {
    ($renderer:ty, $app:ty) => {
        #[cfg(target_os = "android")]
        #[no_mangle]
        pub fn android_main(app: AndroidApp) {
            
            WindowApp::<BaseApp<$renderer, $app>>::new(app_storage_path!()).start(app);
        }

        #[cfg(target_os = "ios")]
        #[no_mangle]
        pub extern "C" fn ios_main() {
            WindowApp::<BaseApp<$renderer, $app>>::new(app_storage_path!()).start();
        }

        #[cfg(target_arch = "wasm32")]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
        pub fn wasm_main() {
            WindowApp::<BaseApp<$renderer, $app>>::new(app_storage_path!()).start();
        }

        #[cfg(not(any(target_os = "android", target_os="ios", target_arch = "wasm32")))]
        pub fn desktop_main() {
            if std::env::args().len() == 1 {
                WindowApp::<BaseApp<$renderer, $app>>::new(app_storage_path!()).start();
            } else {
                BackgroundApp::new_start::<$renderer, $app>(app_storage_path!());
            }
        }
    };
}

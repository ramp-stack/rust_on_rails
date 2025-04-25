use std::future::Future;
use std::path::PathBuf;

pub mod driver;
use driver::logger::Logger;
use driver::state::State;
use driver::cache::Cache;
use driver::camera::Camera;
use driver::runtime::{Runtime, Tasks};

pub mod window;

pub mod renderer;
pub use renderer::Renderer;
pub use renderer::*;

pub trait BaseAppTrait<R: Renderer> {
    const LOG_LEVEL: log::Level;
    fn background_tasks(ctx: &mut HeadlessContext) -> impl Future<Output = Tasks> where Self: Sized;
    fn new<'a>(
        ctx: &'a mut Context<'a, R>, h_ctx: &mut HeadlessContext, width: f32, height: f32
    ) -> impl Future<Output = (Self, Tasks)> where Self: Sized;
    fn on_event<'a>(&'a mut self, ctx: &'a mut Context<'a, R>, event: R::Event);
    fn close(self) -> impl Future<Output = ()>;
}

#[derive(Debug, Clone)]
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

pub struct Context<'a, R: Renderer> {//CookingContext<'a>
    pub cache: &'a mut Cache,
    pub state: &'a mut State,
    pub r_ctx: &'a mut R::Context//Kitchen
}

impl<R: Renderer> AsMut<R::Context> for Context<'_, R> {
    fn as_mut(&mut self) -> &mut R::Context {self.r_ctx}
}

impl<'a, R: Renderer> Context<'a, R> {
    fn new(cache: &'a mut Cache, state: &'a mut State, r_ctx: &'a mut R::Context) -> Self {
        Context{cache, state, r_ctx}
    }

    pub fn state(&mut self) -> &mut State {self.state}

    pub fn open_camera() -> Camera { Camera::new() }
}

pub struct BackgroundApp;
impl BackgroundApp {
    pub fn new_start<R: Renderer, A: BaseAppTrait<R>>(storage_path: PathBuf) {
        #[cfg(not(target_arch="wasm32"))]
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        #[cfg(not(target_arch="wasm32"))]
        runtime.block_on(async {
            let mut ctx = HeadlessContext::new(storage_path).await;
            let tasks = A::background_tasks(&mut ctx).await;
            Runtime::new_background(ctx, tasks).await;
        });

        #[cfg(target_arch="wasm32")]
        unimplemented!()
    }
}

///BaseApp is the heart of rust_on_rails providing all
///of the hardware interfaces for higher level applications(canvas, components)
pub struct BaseApp<R: Renderer, A: BaseAppTrait<R>> {
    _p: std::marker::PhantomData<R>,
    runtime: Runtime,
    cache: Cache,
    state: State,
    app: A
}

impl<R: Renderer, A: BaseAppTrait<R>> RenderAppTrait<R> for BaseApp<R, A> {
    async fn new(
        storage_path: PathBuf, ctx: &mut R::Context, width: f32, height: f32
    ) -> Self {
        Logger::start(A::LOG_LEVEL);        
        let mut headless_ctx = HeadlessContext::new(storage_path.clone()).await;
        let mut cache = Cache::new(storage_path).await;
        let mut state = State::default();
        let background_tasks = if cfg!(any(target_os = "ios", target_os = "android")) {
            A::background_tasks(&mut headless_ctx).await
        } else {vec![]};
        let mut ctx = Context::new(&mut cache, &mut state, ctx);
        let (app, tasks) = A::new(&mut ctx, &mut headless_ctx, width, height).await;
        let runtime = Runtime::new(headless_ctx, background_tasks, tasks).await;
        BaseApp{
            _p: std::marker::PhantomData::<R>,
            runtime, cache, state, app
        }
    }
    async fn on_event(&mut self, ctx: &mut R::Context, event: R::Event) {
        if event.is_paused() {self.runtime.pause();}
        if event.is_resumed() {self.runtime.resume();}
        let mut ctx = Context::new(&mut self.cache, &mut self.state, ctx);
        self.app.on_event(&mut ctx, event);
    }

    async fn close(mut self, _ctx: &mut R::Context) {self.app.close().await; self.runtime.close()}
}

#[macro_export]
macro_rules! create_base_entry_points {
    ($renderer:ty, $app:ty) => {
        #[cfg(target_os = "android")]
        #[no_mangle]
        pub fn android_main(app: AndroidApp) {
            
            WindowApp::<RenderApp<$renderer, BaseApp<$renderer, $app>>>::new(app_storage_path!()).start(app);
        }

        #[cfg(target_os = "ios")]
        #[no_mangle]
        pub extern "C" fn ios_main() {
            WindowApp::<RenderApp<$renderer, BaseApp<$renderer, $app>>>::new(app_storage_path!()).start();
        }

        #[cfg(target_arch = "wasm32")]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
        pub fn wasm_main() {
            WindowApp::<RenderApp<$renderer, BaseApp<$renderer, $app>>>::new(app_storage_path!()).start();
        }

        #[cfg(not(any(target_os = "android", target_os="ios", target_arch = "wasm32")))]
        pub fn desktop_main() {
            if std::env::args().len() == 1 {
                WindowApp::<RenderApp<$renderer, BaseApp<$renderer, $app>>>::new(app_storage_path!()).start();
            } else {
                BackgroundApp::new_start::<$renderer, $app>(app_storage_path!());
            }
        }
    };
}

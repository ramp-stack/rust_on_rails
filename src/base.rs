use std::future::Future;
use std::sync::Arc;

pub use raw_window_handle::{HasWindowHandle, HasDisplayHandle};

mod logger;
pub use logger::Logger;

mod events;
pub use events::*;

mod state;
pub use state::{State, Field};

mod cache;
pub use cache::Cache;

mod tasks;
pub use tasks::{Scheduler, Thread};

mod app;
pub use app::{BaseBackgroundApp, BaseApp};

mod winit;
pub use winit::*;

pub type Callback = Box<dyn FnOnce(&mut State) + Send>;

pub trait BaseBackgroundAppTrait {
    const LOG_LEVEL: log::Level;

    fn new(ctx: &mut BaseAsyncContext) -> impl Future<Output = Self> where Self: Sized;

    fn on_tick(&mut self, ctx: &mut BaseAsyncContext) -> impl Future<Output = ()>;
}

pub trait BaseAppTrait {
    const LOG_LEVEL: log::Level;

    ///Triggered on app start up
    fn new(ctx: &mut BaseContext) -> impl Future<Output = Self> where Self: Sized;

    ///Triggered whenever the app returns from background
    fn on_resume<W: HasWindowHandle + HasDisplayHandle>(
        &mut self, ctx: &mut BaseContext, window: Arc<W>, width: u32, height: u32, scale_factor: f64
    ) -> impl Future<Output = ()>;

    ///Triggered after every tick that the app is active
    ///WASM: Will be triggered after each tick
    ///Other: Will be triggered on a different thread to prevent frame hangs
    fn on_async_tick(ctx: &mut BaseAsyncContext) -> impl Future<Output = Callback> + Send;

    ///Triggered every tick that the app is resumed
    fn on_tick(&mut self, ctx: &mut BaseContext);
    ///Triggered whenever the app is sent to the background
    fn on_pause(&mut self, ctx: &mut BaseContext);
  /////Triggered every tick that the app is paused
  //fn on_paused_tick(&mut self, ctx: &mut BaseContext);
    ///Triggered whenever the app is closed
    fn on_close(self, ctx: &mut BaseContext);
    ///Triggered on window events
    fn on_event(&mut self, ctx: &mut BaseContext, event: Event);
}

pub struct BaseContext {
    name: String,
    pub state: State,
    pub scheduler: Scheduler<Callback>
}

impl BaseContext {
    pub fn pkg_name(&self) -> &String {&self.name}
    //TODO: pub fn open_camera(...)
}

pub struct BaseAsyncContext {
    pub cache: Cache,
}

impl BaseAsyncContext {
    pub fn new(name: &str) -> BaseAsyncContext {
        BaseAsyncContext{cache: Cache::new(name)}
    }
}

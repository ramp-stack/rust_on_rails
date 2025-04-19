use std::future::Future;
use std::sync::Arc;

use raw_window_handle::{HasWindowHandle, HasDisplayHandle};

mod logger;
pub use logger::Logger;

mod state;
pub use state::{State, Field};

mod cache;
pub use cache::Cache;

mod tasks;
pub use tasks::*;

pub mod camera;


pub trait WindowHandle: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static {}
impl<W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static> WindowHandle for W {}

pub struct AsyncContext {
    pub cache: Cache,
}

pub trait BackgroundApp: Send {
    const LOG_LEVEL: log::Level;

    fn new(ctx: &mut AsyncContext) -> impl Future<Output = Self> where Self: Sized;

    fn register_tasks(&mut self, ctx: &mut AsyncContext) -> impl Future<Output = BackgroundTasks<Self>> where Self: Sized;
}

pub struct BaseContext {
    pub state: State,
}

impl BaseContext {
    //TODO: pub fn open_camera(...)
}

pub trait BaseAppTrait {
    const LOG_LEVEL: log::Level;

    fn register_tasks() -> impl Future<Output = AsyncTasks>;

    fn new<W: WindowHandle>(
        ctx: &mut BaseContext, window: Arc<W>, width: u32, height: u32, scale_factor: f64
    ) -> impl Future<Output = Self> where Self: Sized;

    fn on_resume<W: WindowHandle>(
        &mut self, ctx: &mut BaseContext, window: Arc<W>, width: u32, height: u32, scale_factor: f64
    ) -> impl Future<Output = ()>;

    fn on_event(&mut self, ctx: &mut BaseContext, event: WindowEvent);
}

mod app;
pub use app::*;

//TODO: Replace winit structures with custom structs
pub use winit_crate::keyboard::{NamedKey, SmolStr, Key};

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    Resize{width: u32, height: u32, scale_factor: f64},
    Mouse{position: (u32, u32), state: MouseState},
    Keyboard{key: Key, state: KeyboardState},
    Pause,
    Close,
    Tick
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

#[macro_export]
macro_rules! create_base_entry_points {
    ($app:ty, $bg_app:ty) => {
        pub type BackgroundTasks = Vec<(std::time::Duration, BackgroundTask<$bg_app>)>;

        create_app_entry_points!($app, $bg_app);
    };
}

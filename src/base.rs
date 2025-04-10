pub use raw_window_handle::{HasWindowHandle, HasDisplayHandle};
//  use std::sync::LazyLock;
//  use std::path::Path;

//  static HOME: LazyLock<Path> = LazyLock::new(|| {
//     #[cfg(target_os="linux")]
//      {
//          Path::new(format!("/var/lib/", env!("CARGO_CRATE_NAME")))
//      }

//     #[cfg(not(target_os="linux"))] { unimplement!(); }
//  });

mod winit;
pub use winit::*;

mod state;
pub use state::{State, Field};

//TODO: Replace winit event data with custom event data
pub use winit_crate::keyboard::{NamedKey, SmolStr, Key};

pub trait BaseAppTrait {
    const LOG_LEVEL: log::Level;

    ///Triggered on app start up
    fn new<W: HasWindowHandle + HasDisplayHandle>(
        window: std::sync::Arc<W>, width: u32, height: u32, scale_factor: f64
    ) -> Self where Self: Sized;

    ///Triggered whenever the app returns from background
    fn on_resume<W: HasWindowHandle + HasDisplayHandle>(&mut self, window: std::sync::Arc<W>);
    ///Triggered whenever the app is sent to the background
    fn on_pause(&mut self);
    ///Triggered whenever the app is closed
    fn on_close(self);

    ///Triggered after every tick that the app is active
    fn on_tick(&mut self);
    ///Triggered as often as the OS allows while in the background
    fn on_background_tick();

    ///Triggered whenever the app resizes or changes scale
    fn on_resize(&mut self, width: u32, height: u32, scale_factor: f64);
    ///Triggered on mouse or touch events
    fn on_mouse(&mut self, event: MouseEvent);
    ///Triggered on keyboard events
    fn on_keyboard(&mut self, event: KeyboardEvent);
}

#[derive(Default)]
pub struct BaseContext {
    state: State,
}

impl BaseContext {
    //TODO: pub fn open_camera(...)
}

///The BaseApp is the center of the rust_on_rails system
///Lifetime managers like winit will accept a BaseApp and call the appropriate events
///Hardware Interfaces and Physical Events are provided by the BaseApp
pub struct BaseApp<A: BaseAppTrait + 'static> {
    ctx: BaseContext,
    app: A
}

impl<A: BaseAppTrait + 'static> BaseApp<A> {
    pub const LOG_LEVEL: log::Level = A::LOG_LEVEL;

    pub fn new<W: HasWindowHandle + HasDisplayHandle>(
        window: std::sync::Arc<W>, width: u32, height: u32, scale_factor: f64
    ) -> Self {
        BaseApp{ctx: BaseContext::default(), app: A::new(window, width, height, scale_factor)}
    }

    pub fn on_resume<W: HasWindowHandle + HasDisplayHandle>(&mut self, window: std::sync::Arc<W>) {
        self.app.on_resume(window);
    }
    pub fn on_pause(&mut self) {self.app.on_pause();}
    pub fn on_close(self) {self.app.on_close();}

    pub fn on_tick(&mut self) {self.app.on_tick();}
    pub fn on_background_tick() {A::on_background_tick();}

    pub fn on_resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.app.on_resize(width, height, scale_factor);
    }
    pub fn on_mouse(&mut self, event: MouseEvent) {self.app.on_mouse(event);}
    pub fn on_keyboard(&mut self, event: KeyboardEvent) {self.app.on_keyboard(event);}
}

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

#[macro_export]
macro_rules! create_base_entry_points {
    ($app:ty) => {
        create_winit_entry_points!($app);
    };
}

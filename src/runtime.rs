use crate::base::*;

#[cfg(not(target_arch="wasm32"))]
use std::sync::mpsc::{Receiver, Sender};

#[cfg(target_arch="wasm32")]
use std::sync::Mutex;

use std::future::Future;
use std::sync::Arc;

pub type Callback<A> = Box<dyn FnMut(&mut A) + Send>;

pub trait RuntimeAppTrait {
    const LOG_LEVEL: log::Level = log::Level::Error;

    ///Triggered on app start up
    fn new<W: HasWindowHandle + HasDisplayHandle>(
        window: Arc<W>, width: u32, height: u32, scale_factor: f64
    ) -> impl Future<Output = Self> where Self: Sized;

    ///Triggered whenever the app returns from background
    fn on_resume<W: HasWindowHandle + HasDisplayHandle>(&mut self, window: Arc<W>);
    ///Triggered whenever the app is sent to the background
    fn on_pause(&mut self);
    ///Triggered whenever the app is closed
    fn on_close(self);

    ///Triggered every tick that the app is to be rendered
    fn on_tick(&mut self);

    ///Triggered after every tick that the app is active
    ///WASM: Will be triggered after each tick and never as background
    ///Other: Will be triggered on a different thread to prevent frame hangs and
    ///for background will be triggered as often as possible per OS.
    ///
    ///Suggested to use a field in a database for regular timing of this function
    fn on_async_tick(background: bool) -> impl Future<Output = Option<Callback<Self>>> + Send;

    ///Triggered whenever the app resizes or changes scale
    fn on_resize(&mut self, width: u32, height: u32, scale_factor: f64);
    ///Triggered on mouse or touch events
    fn on_mouse(&mut self, event: MouseEvent);
    ///Triggered on keyboard events
    fn on_keyboard(&mut self, event: KeyboardEvent);
}

#[cfg(not(target_arch="wasm32"))]
pub struct RuntimeApp<A: RuntimeAppTrait + 'static> {
    callbacks: Receiver<Callback<A>>,
    runtime: tokio::runtime::Runtime,
    app: A,
}

#[cfg(not(target_arch="wasm32"))]
impl<A: RuntimeAppTrait + 'static> RuntimeApp<A> {
    async fn async_loop(c_sender: Sender<Callback<A>>) {
        loop {
            if let Some(callback) = A::on_async_tick(false).await {
                if c_sender.send(callback).is_err() {return;}
            }
        }
    }
}

#[cfg(not(target_arch="wasm32"))]
impl<A: RuntimeAppTrait + 'static> BaseAppTrait for RuntimeApp<A> {
    const LOG_LEVEL: log::Level = A::LOG_LEVEL;

    fn new<W: HasWindowHandle + HasDisplayHandle>(
        window: std::sync::Arc<W>, width: u32, height: u32, scale_factor: f64
    ) -> Self {
        let (c_sender, callbacks) = std::sync::mpsc::channel();
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).build().unwrap();
        runtime.spawn(Self::async_loop(c_sender));
        let app = runtime.block_on(A::new(window, width, height, scale_factor));
        RuntimeApp{callbacks, runtime, app}
    }

    fn on_resume<W: HasWindowHandle + HasDisplayHandle>(&mut self, win: std::sync::Arc<W>) {
        self.app.on_resume(win);
    }
    fn on_pause(&mut self) {
        self.app.on_pause();
    }
    fn on_close(self) {
        self.runtime.shutdown_background();
        self.app.on_close();
    }

    fn on_tick(&mut self) {
        self.callbacks.try_iter().for_each(|mut cb| cb(&mut self.app));
        self.app.on_tick();
    }

    fn on_background_tick() {
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        runtime.block_on(A::on_async_tick(true));
    }

    fn on_resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.app.on_resize(width, height, scale_factor);
    }
    fn on_mouse(&mut self, event: MouseEvent) {self.app.on_mouse(event);}
    fn on_keyboard(&mut self, event: KeyboardEvent) {self.app.on_keyboard(event);}
}

#[cfg(target_arch="wasm32")]
pub struct RuntimeApp<A: RuntimeAppTrait + 'static> {
    lf: Option<LocalFuture<A>>,
    app: Option<A>,
}

#[cfg(target_arch="wasm32")]
impl<A: RuntimeAppTrait + 'static> RuntimeApp<A> {
    fn app(&mut self) -> &mut A {
        if let Some(lf) = self.lf.take() {
            self.app = Some(lf.unwrap());
        }
        self.app.as_mut().unwrap()
    }

    pub fn spawn_local<T: 'static>(task: impl Future<Output = T>) -> LocalFuture<T> {
        let lock = Arc::new(Mutex::new(None));
        let local_future = LocalFuture{lock: lock.clone()};
        let future = async move { *lock.lock().unwrap() = Some(task.await); };

        wasm_bindgen_futures::spawn_local(future);

        local_future
    }
}

#[cfg(target_arch="wasm32")]
impl<A: RuntimeAppTrait + 'static> BaseAppTrait for RuntimeApp<A> {
    const LOG_LEVEL: log::Level = A::LOG_LEVEL;

    fn new<W: HasWindowHandle + HasDisplayHandle>(
        window: std::sync::Arc<W>, width: u32, height: u32, scale_factor: f64
    ) -> Self {
        let lf = Self::spawn_local(A::new(window, width, height, scale_factor));
        RuntimeApp{lf: Some(lf), app: None}
    }

    fn on_resume<W: HasWindowHandle + HasDisplayHandle>(&mut self, win: std::sync::Arc<W>) {
        self.app().on_resume(win);
    }
    fn on_pause(&mut self) {
        self.app().on_pause();
    }
    fn on_close(&mut self) {
        self.app().on_close();
    }

    fn on_tick(&mut self) {
        self.app().on_tick();
        let app = self.app.take().unwrap();
        self.lf = Some(self.runtime.spawn_local(async move {
            let cb = A::on_async_tick(false); if let Some(cb) = cb {cb(&mut app);} app
        }));
    }

    fn on_background_tick() {}

    fn on_resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.app().on_resize(width, height, scale_factor);
    }
    fn on_mouse(&mut self, event: MouseEvent) {self.app().on_mouse(event);}
    fn on_keyboard(&mut self, event: KeyboardEvent) {self.app().on_keyboard(event);}
}

#[cfg(target_arch="wasm32")]
pub struct LocalFuture<T> {
    lock: Arc<Mutex<Option<T>>>,
}

#[cfg(target_arch="wasm32")]
impl<T> LocalFuture<T> {
    pub fn unwrap(self) -> T {
        Arc::into_inner(self.lock).unwrap().into_inner().unwrap().expect("LocalFuture did not complete")
    }
}

#[macro_export]
macro_rules! create_runtime_entry_points {
    ($app:ty) => {
        create_base_entry_points!(RuntimeApp::<$app>);
    };
}

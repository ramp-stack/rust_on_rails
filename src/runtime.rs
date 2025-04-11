use serde::{Serialize, Deserialize};

use crate::base::*;
use std::time::{Instant, Duration};

#[cfg(not(target_arch="wasm32"))]
use std::sync::mpsc::{Receiver, Sender};
#[cfg(not(target_arch="wasm32"))]
use std::sync::mpsc::TryRecvError;

#[cfg(target_arch="wasm32")]
use std::sync::Mutex;

use std::future::Future;
use std::sync::Arc;

pub type Callback = Box<dyn FnMut(&mut State) + Send>;

pub type RuntimeContext = BaseContext;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Snapshot(State);

pub struct RuntimeAsyncContext {
    pub cache: Cache,
}

pub trait RuntimeBGAppTrait {
    const LOG_LEVEL: log::Level = log::Level::Error;
    fn new(tx: &mut RuntimeAsyncContext) -> impl Future<Output = Self> where Self: Sized;

    ///Runs in the background as and while the app is paused in place of async_tick
    fn on_tick(&mut self, ctx: &mut RuntimeAsyncContext) -> impl Future<Output = ()>;
}


pub trait RuntimeAppTrait {
    const LOG_LEVEL: log::Level = log::Level::Error;

    ///Triggered on app start up
    fn new(ctx: &mut RuntimeContext) -> impl Future<Output = Self> where Self: Sized;

    ///Triggered whenever the app returns from background
    fn on_resume<W: HasWindowHandle + HasDisplayHandle>(
        &mut self, ctx: &mut RuntimeContext, window: Arc<W>, width: u32, height: u32, scale_factor: f64
    ) -> impl Future<Output = ()>;

    ///Triggered after every tick that the app is active
    ///WASM: Will be triggered after each tick
    ///Other: Will be triggered on a different thread to prevent frame hangs
    fn on_async_tick(ctx: &mut RuntimeAsyncContext) -> impl Future<Output = Callback> + Send;

    ///Triggered every tick that the app is to be rendered
    fn on_tick(&mut self, ctx: &mut RuntimeContext);

    ///Triggered whenever the app is sent to the background
    fn on_pause(&mut self, ctx: &mut RuntimeContext);

    ///Triggered whenever the app is closed
    fn on_close(self, ctx: &mut RuntimeContext);

    ///Triggered on window events
    fn on_event(&mut self, ctx: &mut RuntimeContext, event: Event);
}

#[cfg(not(target_arch="wasm32"))]
pub struct RuntimeApp<A: RuntimeAppTrait + 'static> {
    callbacks: Receiver<Callback>,
    s_sender: Sender<bool>,
    runtime: tokio::runtime::Runtime,
    app: A,
}

#[cfg(not(target_arch="wasm32"))]
impl<A: RuntimeAppTrait + 'static> RuntimeApp<A> {
    async fn async_loop(
        mut ctx: RuntimeAsyncContext, c_sender: Sender<Callback>, s_receiver: Receiver<bool>
    ) {
        let mut paused = false;
        let mut last_tick = Instant::now();
        loop {
            if last_tick.elapsed() > THREAD_TICK {
                last_tick = Instant::now();
                if paused {
                    match s_receiver.recv() {
                        Ok(status) if status => paused = false,
                        _ => return,
                    };
                } else {
                    match s_receiver.try_recv() {
                        Ok(status) if status => paused = true,
                        Err(TryRecvError::Empty) => {
                            let callback = A::on_async_tick(&mut ctx).await;
                            if c_sender.send(callback).is_err() {return;}
                            std::thread::sleep(std::time::Duration::from_millis(15));
                        }
                        _ => return,
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch="wasm32"))]
impl<A: RuntimeAppTrait + 'static> BaseAppTrait for RuntimeApp<A> {
    const LOG_LEVEL: log::Level = A::LOG_LEVEL;

    fn new(ctx: &mut BaseContext) -> Self {
        let (c_sender, callbacks) = std::sync::mpsc::channel();
        let (s_sender, s_receiver) = std::sync::mpsc::channel();
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).build().unwrap();
        let cache = runtime.block_on(Cache::new(ctx));
        ctx.state = runtime.block_on(cache.get::<Snapshot>()).0;
        runtime.spawn(Self::async_loop(RuntimeAsyncContext{cache}, c_sender, s_receiver));
        let app = runtime.block_on(A::new(ctx));
        RuntimeApp{callbacks, s_sender, runtime, app}
    }

    fn new_background(ctx: &mut BaseContext) {
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let mut ctx = RuntimeAsyncContext{cache: runtime.block_on(Cache::new(ctx))};
        runtime.block_on(A::on_background(&mut ctx));
    }

    fn on_resume<W: HasWindowHandle + HasDisplayHandle>(
        &mut self, ctx: &mut BaseContext, window: Arc<W>, width: u32, height: u32, scale_factor: f64
    ) {
        self.s_sender.send(true).unwrap();
        self.runtime.block_on(self.app.on_resume(ctx, window, width, height, scale_factor));
    }

    fn on_tick(&mut self, ctx: &mut BaseContext) {
        self.callbacks.try_iter().for_each(|mut cb| cb(&mut ctx.state));
        self.app.on_tick(ctx);
    }

    fn on_pause(&mut self, ctx: &mut BaseContext) {
        self.s_sender.send(true).unwrap();
        self.app.on_pause(ctx);
    }

    fn on_close(self, mut ctx: BaseContext) {
        self.s_sender.send(false).unwrap();
        self.app.on_close(&mut ctx);
        let cache = self.runtime.block_on(Cache::new(&mut ctx));
        self.runtime.block_on(cache.set(Snapshot(ctx.state)));
        self.runtime.shutdown_background();
    }

    fn on_event(&mut self, ctx: &mut BaseContext, event: Event) {
        self.app.on_event(ctx, event);
    }
}

//  #[cfg(target_arch="wasm32")]
//  pub struct RuntimeApp<A: RuntimeAppTrait + 'static> {
//      lf: Option<LocalFuture<A>>,
//      app: Option<A>,
//  }

//  #[cfg(target_arch="wasm32")]
//  impl<A: RuntimeAppTrait + 'static> RuntimeApp<A> {
//      fn app(&mut self) -> &mut A {
//          if let Some(lf) = self.lf.take() {
//              self.app = Some(lf.unwrap());
//          }
//          self.app.as_mut().unwrap()
//      }

//      pub fn spawn_local<T: 'static>(task: impl Future<Output = T>) -> LocalFuture<T> {
//          let lock = Arc::new(Mutex::new(None));
//          let local_future = LocalFuture{lock: lock.clone()};
//          let future = async move { *lock.lock().unwrap() = Some(task.await); };

//          wasm_bindgen_futures::spawn_local(future);

//          local_future
//      }
//  }

//  #[cfg(target_arch="wasm32")]
//  impl<A: RuntimeAppTrait + 'static> BaseAppTrait for RuntimeApp<A> {
//      const LOG_LEVEL: log::Level = A::LOG_LEVEL;

//      fn new<W: HasWindowHandle + HasDisplayHandle>(
//          window: std::sync::Arc<W>, width: u32, height: u32, scale_factor: f64
//      ) -> Self {
//          let lf = Self::spawn_local(A::new(window, width, height, scale_factor));
//          RuntimeApp{lf: Some(lf), app: None}
//      }

//      fn on_resume<W: HasWindowHandle + HasDisplayHandle>(&mut self, win: std::sync::Arc<W>) {
//          self.app().on_resume(win);
//      }
//      fn on_pause(&mut self) {
//          self.app().on_pause();
//      }
//      fn on_close(&mut self) {
//          self.app().on_close();
//      }

//      fn on_tick(&mut self) {
//          self.app().on_tick();
//          let app = self.app.take().unwrap();
//          self.lf = Some(self.runtime.spawn_local(async move {
//              let cb = A::on_async_tick(false); if let Some(cb) = cb {cb(&mut app);} app
//          }));
//      }

//      fn on_background_tick() {}

//      fn on_resize(&mut self, width: u32, height: u32, scale_factor: f64) {
//          self.app().on_resize(width, height, scale_factor);
//      }
//      fn on_mouse(&mut self, event: MouseEvent) {self.app().on_mouse(event);}
//      fn on_keyboard(&mut self, event: KeyboardEvent) {self.app().on_keyboard(event);}
//  }

//  #[cfg(target_arch="wasm32")]
//  pub struct LocalFuture<T> {
//      lock: Arc<Mutex<Option<T>>>,
//  }

//  #[cfg(target_arch="wasm32")]
//  impl<T> LocalFuture<T> {
//      pub fn unwrap(self) -> T {
//          Arc::into_inner(self.lock).unwrap().into_inner().unwrap().expect("LocalFuture did not complete")
//      }
//  }

#[macro_export]
macro_rules! create_runtime_entry_points {
    ($app:ty) => {
        create_base_entry_points!(RuntimeApp::<$app>);
    };
}

//  use crate::State;
//  use std::sync::mpsc::{Receiver, Sender, SyncSender};
//  use std::future::Future;
//  use std::time::Duration;
//  use std::sync::{Mutex, Arc};

//  pub struct RunningFuture<T> {
//      lock: Arc<Mutex<Option<T>>>,
//      channel: Receiver<bool>
//  }

//  impl<T> RunningFuture<T> {
//      pub fn blocking_await(self, timeout: Duration) -> T {
//          self.channel.recv_timeout(timeout).unwrap();
//          Arc::into_inner(self.lock).unwrap().into_inner().unwrap().unwrap()
//      }
//  }

//  pub struct Runtime {
//      #[cfg(not(target_arch="wasm32"))]
//      runtime: tokio::runtime::Runtime,
//  }

//  impl Runtime {
//      pub fn new() -> Self {
//          Runtime{
//              #[cfg(not(target_arch="wasm32"))]
//              runtime: tokio::runtime::Builder::new_multi_thread().worker_threads(1).build().unwrap(),
//          }
//      }

//      pub fn exit(self) {
//          #[cfg(not(target_arch="wasm32"))]
//          self.runtime.shutdown_background();
//      }

//      pub fn spawn_local<T: 'static>(
//          &self, task: impl Future<Output = T> + 'static
//      ) -> RunningFuture<T> {
//          let lock = Arc::new(Mutex::new(None));
//          let (sender, receiver) = std::sync::mpsc::sync_channel(1);
//          let running_future = RunningFuture{lock: lock.clone(), channel: receiver};
//          let future = async move { *lock.lock().unwrap() = Some(task.await); sender.send(true).unwrap(); };

//          #[cfg(not(target_arch="wasm32"))]
//          self.runtime.block_on(future);
//          #[cfg(target_arch="wasm32")]
//          wasm_bindgen_futures::spawn_local(future);

//          running_future
//      }

//      pub fn spawn<T: 'static + Send>(
//          &self, task: impl Future<Output = T> + Send + 'static
//      ) -> Receiver<T> {
//          let (sender, receiver) = std::sync::mpsc::sync_channel(1);
//          let future = async move { sender.send(task.await).unwrap(); };

//          #[cfg(not(target_arch="wasm32"))]
//          self.runtime.spawn(future);
//          #[cfg(target_arch="wasm32")]
//          wasm_bindgen_futures::spawn_local(future);

//          receiver
//      }
//  }

//  pub type Callback = Box<dyn FnOnce(&mut State) + Send>;
//  pub type BFuture = std::pin::Pin<Box<dyn Future<Output = (Option<Duration>, Callback)> + Send>>;
//  pub type Function = Box<dyn Fn() -> BFuture + Send>;

//  #[derive(Clone, Debug)]
//  pub struct Scheduler {
//      sender: Sender<(Duration, Function)>
//  }

//  impl Scheduler {
//      pub fn schedule_task<
//          F: Fn() -> Fut + Send + 'static,
//          Fut: Future<Output = (Option<Duration>, Callback)> + 'static + Send
//      >(
//          &self, duration: Duration, task: F
//      ) {
//          self.sender.send((duration, Box::new(move || Box::pin(task()) as BFuture) as Function)).unwrap();
//      }
//  }

//  pub struct TaskManager {
//      callbacks: Receiver<Callback>,
//  }

//  impl TaskManager {
//      pub fn new(runtime: &Runtime) -> (Self, Scheduler) {
//          let (c_sender, callbacks) = std::sync::mpsc::sync_channel(100);
//          let (sender, receiver) = std::sync::mpsc::channel();
//          runtime.spawn(Self::process_async(receiver, c_sender));

//          (
//              TaskManager{callbacks},
//              Scheduler{sender}
//          )
//      }

//      pub fn callbacks(&self) -> Vec<Callback> {self.callbacks.try_iter().collect()}

//      async fn process_async(
//          rc: Receiver<(Duration, Function)>,
//          tx: SyncSender<Callback>
//      ) {
//          let time = std::time::Instant::now();
//          let mut tasks = Vec::new();
//          loop {
//              while let Ok((duration, function)) = rc.try_recv() {
//                  tasks.push((duration, function));
//              }
//              for (duration, function) in std::mem::take(&mut tasks) {
//                  if duration < time.elapsed() {
//                      let (duration, callback) = function().await;
//                      tx.send(callback).unwrap();
//                      if let Some(duration) = duration {
//                          tasks.push((time.elapsed()+duration, function));
//                      }
//                  } else {
//                      tasks.push((duration, function));
//                  }
//              }
//              std::thread::sleep(Duration::from_millis(1))
//          }
//      }
//  }

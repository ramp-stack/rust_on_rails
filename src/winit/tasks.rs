use crate::State;
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::future::Future;
use std::time::Duration;
use std::sync::{Mutex, Arc};

pub struct RunningFuture<T> {
    lock: Arc<Mutex<Option<T>>>,
    channel: Receiver<bool>
}

impl<T> RunningFuture<T> {
    pub fn blocking_await(self, timeout: Duration) -> T {
        self.channel.recv_timeout(timeout).unwrap();
        Arc::into_inner(self.lock).unwrap().into_inner().unwrap().unwrap()
    }
}

pub struct Runtime {
    #[cfg(not(target_arch="wasm32"))]
    runtime: tokio::runtime::Runtime,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime{
            #[cfg(not(target_arch="wasm32"))]
            runtime: tokio::runtime::Builder::new_multi_thread().worker_threads(1).build().unwrap(),
        }
    }

    pub fn exit(self) {
        #[cfg(not(target_arch="wasm32"))]
        self.runtime.shutdown_background();
    }

    pub fn spawn_local<T: 'static>(
        &self, task: impl Future<Output = T> + 'static
    ) -> RunningFuture<T> {
        let lock = Arc::new(Mutex::new(None));
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let running_future = RunningFuture{lock: lock.clone(), channel: receiver};
        let future = async move { *lock.lock().unwrap() = Some(task.await); sender.send(true).unwrap(); };

        #[cfg(not(target_arch="wasm32"))]
        self.runtime.block_on(future);
        #[cfg(target_arch="wasm32")]
        wasm_bindgen_futures::spawn_local(future);

        running_future
    }

    pub fn spawn<T: 'static + Send>(
        &self, task: impl Future<Output = T> + Send + 'static
    ) -> Receiver<T> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let future = async move { sender.send(task.await).unwrap(); };

        #[cfg(not(target_arch="wasm32"))]
        self.runtime.spawn(future);
        #[cfg(target_arch="wasm32")]
        wasm_bindgen_futures::spawn_local(future);

        receiver
    }
}

pub type Callback = Box<dyn FnOnce(&mut State) + Send>;
pub type BFuture = std::pin::Pin<Box<dyn Future<Output = (Option<Duration>, Callback)> + Send>>;
pub type Function = Box<dyn Fn() -> BFuture + Send>;

#[derive(Clone, Debug)]
pub struct Scheduler {
    sender: Sender<(Duration, Function)>
}

impl Scheduler {
    pub fn schedule_task<
        F: Fn() -> Fut + Send + 'static,
        Fut: Future<Output = (Option<Duration>, Callback)> + 'static + Send
    >(
        &self, duration: Duration, task: F
    ) {
        self.sender.send((duration, Box::new(move || Box::pin(task()) as BFuture) as Function)).unwrap();
    }
}

pub struct TaskManager {
    callbacks: Receiver<Callback>,
}

impl TaskManager {
    pub fn new(runtime: &Runtime) -> (Self, Scheduler) {
        let (c_sender, callbacks) = std::sync::mpsc::sync_channel(100);
        let (sender, receiver) = std::sync::mpsc::channel();
        runtime.spawn(Self::process_async(receiver, c_sender));

        (
            TaskManager{callbacks},
            Scheduler{sender}
        )
    }

    pub fn callbacks(&self) -> Vec<Callback> {self.callbacks.try_iter().collect()}

    async fn process_async(
        rc: Receiver<(Duration, Function)>,
        tx: SyncSender<Callback>
    ) {
        let time = std::time::Instant::now();
        let mut tasks = Vec::new();
        loop {
            while let Ok((duration, function)) = rc.try_recv() {
                tasks.push((duration, function));
            }
            for (duration, function) in std::mem::take(&mut tasks) {
                if duration < time.elapsed() {
                    let (duration, callback) = function().await;
                    tx.send(callback).unwrap();
                    if let Some(duration) = duration {
                        tasks.push((time.elapsed()+duration, function));
                    }
                } else {
                    tasks.push((duration, function));
                }
            }
            std::thread::sleep(Duration::from_millis(1))
        }
    }
}

use std::future::Future;
use std::sync::{Mutex, Arc};
use std::time::{Instant, Duration};
use std::sync::mpsc::{TryRecvError, Sender, Receiver, channel};

pub const THREAD_TICK: u64 = 16;

use super::{HeadlessContext, Renderer, BaseAppTrait};
pub use async_trait::async_trait;

#[derive(Default)]
pub struct BlockingFuture<T: 'static>(Arc<Mutex<Option<T>>>);
impl<T: 'static> BlockingFuture<T> {
    pub fn unwrap(self) -> T {
        Arc::into_inner(self.0).unwrap().into_inner().unwrap().unwrap()
    }
}

pub struct BlockingRuntime;
impl BlockingRuntime {
    pub fn block_on<T: 'static>(task: impl Future<Output = T>) -> BlockingFuture<T> {
        #[cfg(not(target_arch="wasm32"))]
        let future = BlockingFuture(Arc::new(Mutex::new(Some(
            //Take current thread and block untill future completes
            tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(task)
        ))));

        #[cfg(target_arch="wasm32")]
        let future = BlockingFuture::default();

        #[cfg(target_arch="wasm32")]
        let arcm = future.0.clone();
        #[cfg(target_arch="wasm32")]
        wasm_bindgen_futures::spawn_local(async move {*arcm.lock().unwrap() = Some(task.await);});

        future
    }
}

#[async_trait]
pub trait Task: Send {
    fn interval(&self) -> Option<Duration>;
    async fn run(&mut self, ctx: &mut HeadlessContext);
}

pub type Tasks = Vec<Box<dyn Task>>;

#[macro_export]
macro_rules! tasks {
    [$($renderer:expr),+] => {
        vec![$(Box::new($renderer) as Box<dyn Task>),+]
    };
}

pub struct TaskManager {
    ctx: HeadlessContext,
    tasks: Vec<(Box<dyn Task>, Instant)> 
}

impl TaskManager {
    pub fn new(ctx: HeadlessContext, tasks: Tasks) -> Self {
        TaskManager{ctx, tasks: tasks.into_iter().map(|t| (t, Instant::now())).collect()}
    }

    pub async fn tick(&mut self) {
        for (task, time) in self.tasks.iter_mut() {
            if let Some(duration) = task.interval() {
                if time.elapsed() > duration {
                    *time = Instant::now();
                    task.run(&mut self.ctx).await;
                }
            }
        }
    }
}

//TODO: Figure out Wasm Web Workers
///Runs Active Tasks (and Background Tasks on mobile/wasm)
pub struct Runtime {
    runtime: Option<tokio::runtime::Runtime>,
    active_thread: Sender<u8>,
}

impl Runtime {
    pub fn new_background(ctx: HeadlessContext, tasks: Tasks) {
        BlockingRuntime::block_on(Self::background_thread(TaskManager::new(ctx, tasks)));
    }

    pub fn new<R: Renderer, A: BaseAppTrait<R>>(
        ctx: HeadlessContext, tasks: Tasks
    ) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).build().unwrap();
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            let tasks = runtime.block_on(A::background_tasks(&mut ctx));
            let task_manager = TaskManager::new(ctx.clone(), tasks);
            runtime.spawn(Self::background_thread(task_manager))
        }

        ;
        let (active_thread, receiver) = channel();
        runtime.spawn(Self::thread(TaskManager::new(ctx, tasks), receiver));

        Runtime{runtime: Some(runtime), active_thread}
    }

    async fn thread(mut task_manager: TaskManager, receiver: Receiver<u8>) {
        let mut paused = false;
        loop {
            if !paused {
                match receiver.try_recv() {
                    Ok(0) => {},
                    Ok(1) => {paused = true}
                    Err(TryRecvError::Empty) => {
                        task_manager.tick().await;
                    },
                    _ => {return;}
                }
            } else {
                match receiver.recv() {
                    Ok(0) => {paused = false;},
                    Ok(1) => {},
                    _ => {return;}
                }
            }
            std::thread::sleep(Duration::from_millis(THREAD_TICK));
        }
    }

    async fn background_thread(mut task_manager: TaskManager) {
        loop {
            task_manager.tick().await;
            std::thread::sleep(Duration::from_millis(THREAD_TICK));
        }
    }

    pub fn pause(&mut self) {
        self.active_thread.send(1).unwrap();
    }

    pub fn resume(&mut self) {
        self.active_thread.send(0).unwrap();
    }

    pub fn close(&mut self) {
        self.runtime.take().unwrap().shutdown_background();
    }
}

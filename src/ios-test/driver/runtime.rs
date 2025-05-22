use std::time::{Instant, Duration};
use std::sync::mpsc::{TryRecvError, Sender, Receiver, channel};

use crate::base::HeadlessContext;

const THREAD_TICK: u64 = 16;

pub use async_trait::async_trait;

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
    pub async fn new_background(ctx: HeadlessContext, tasks: Tasks) {
        Self::background_thread(TaskManager::new(ctx, tasks)).await
    }

    pub async fn new(
        ctx: HeadlessContext, background_tasks: Tasks, tasks: Tasks
    ) -> Self {
        let threads = if cfg!(any(target_os = "ios", target_os = "android")) {2} else {1};
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_time().enable_io().worker_threads(threads).build().unwrap();

        if !background_tasks.is_empty() {
            let task_manager = TaskManager::new(ctx.clone(), background_tasks);
            runtime.spawn(Self::background_thread(task_manager));
        }

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

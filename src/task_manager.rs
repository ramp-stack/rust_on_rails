
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
        runtime.spawn_blocking(Self::process_async(receiver, c_sender));

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

//  use std::sync::mpsc::{Receiver, Sender};
//  use std::time::Duration;
//  use std::sync::Arc;

//  use crate::State;

//  mod tasks;
//  pub use tasks::{Runtime, RunningFuture, TaskManager, Scheduler, Callback};
//      app_inbox: Option<LocalFuture<A>>,
//      runtime: Option<Runtime>,
//      task_manager: Option<TaskManager>,
//  self.app = Some(self.app_inbox.take().unwrap().blocking_await(Duration::from_secs(1)));
//  fn init(&mut self, width: u32, height: u32, scale_factor: f64) {
//          let (task_manager, scheduler) = TaskManager::new(self.runtime.as_ref().unwrap());
//          self.task_manager = Some(task_manager);
//          self.app_inbox = Some(self.runtime.as_ref().unwrap().spawn_local(A::new(
//              self.window(), scheduler, width, height, scale_factor
//          )));
//      }
//  for callback in self.task_manager.as_ref().unwrap().callbacks() {
//                          app.process_callback(callback);
//                      }
//                      app.prepare();
//                      app.render();

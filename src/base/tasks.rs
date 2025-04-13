use std::time::{Instant, Duration};

use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc::{SendError, TryRecvError};

use std::future::Future;
use std::pin::Pin;

use super::{AsyncContext, State};

pub type Callback = Box<dyn FnOnce(&mut State) + Send>;
type BoxFuture<'a, R> = Pin<Box<dyn Future<Output = R> + Send + 'a>>;
type BoxFunction<P, R> = Box<dyn for<'a> FnMut(&'a mut P) -> BoxFuture<'a, R> + Send>;
pub type AsyncTask = BoxFunction<AsyncContext, Callback>;
pub type AsyncTasks = Vec<(Duration, AsyncTask)>;
pub type BackgroundTask<BA> = BoxFunction<(BA, AsyncContext), ()>;
pub(crate) type BackgroundTasks<BA> = Vec<(std::time::Duration, BackgroundTask<BA>)>;

pub struct ThreadHandle<R: Send + 'static> {
    results: Receiver<R>,
    status_s: Sender<u8>,
}

impl<R: Send + 'static> ThreadHandle<R> {
    pub fn results(&self) -> Vec<R> {self.results.try_iter().collect()}

    pub fn resume(&self) {self.status_s.send(0).unwrap();}
    pub fn pause(&self) {self.status_s.send(1).unwrap();}
    pub fn close(self) {self.status_s.send(2).unwrap();}
}

pub struct Thread<P, R: Send + 'static> {
    param: P,
    status_r: Receiver<u8>,
    result_s: Sender<R>,
    tasks: Vec<(Instant, Duration, BoxFunction<P, R>)>,
}

impl<P, R: Send + 'static> Thread<P, R> {
    pub fn new(param: P, tasks: Vec<(Duration, BoxFunction<P, R>)>) -> (Self, ThreadHandle<R>) {
        let (result_s, results) = std::sync::mpsc::channel();
        let (status_s, status_r) = std::sync::mpsc::channel();
        (
            Thread{
                param, status_r, result_s,
                tasks: tasks.into_iter().map(|(d, f)|(Instant::now(), d, f)).collect()
            },
            ThreadHandle{results, status_s}
        )
    }

    pub async fn async_tick(&mut self) -> Result<(), SendError<R>> {
        for (last_run, duration, task) in &mut self.tasks {
            if last_run.elapsed() > *duration {
                *last_run = Instant::now();
                self.result_s.send(task(&mut self.param).await)?;
            }
        }
        Ok(())
    }

    pub async fn async_loop(mut self) {
        let mut paused = false;
        loop {
            if paused {
                match self.status_r.recv() {
                    Ok(0) => paused = false,
                    Ok(1) => {},
                    _ => return,
                };
            } else {
                match self.status_r.try_recv() {
                    Ok(0) => {},
                    Ok(1) => paused = false,
                    Err(TryRecvError::Empty) if self.async_tick().await.is_ok() => {},
                    _ => return
                }
            }
            std::thread::sleep(Duration::from_millis(100))
        }
    }
}

#[macro_export]
macro_rules! async_task {
    ($dur:expr, $task:expr) => {{
        let task: AsyncTask = Box::new(|p: &mut AsyncContext| Box::pin($task(p)));
        ($dur, task)
    }};
}

#[macro_export]
macro_rules! background_task {
    ($dur:expr, $task:expr) => {{
        let task: BackgroundTask<Self> = Box::new(|p: &mut (Self, AsyncContext)| Box::pin($task(&mut p.0, &mut p.1)));
        ($dur, task)
    }};
}

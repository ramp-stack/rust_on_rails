use std::time::{Instant, Duration};

use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc::TryRecvError;

use std::future::Future;

pub type BFuture<T> = std::pin::Pin<Box<dyn Future<Output = (Option<Duration>, T)> + Send>>;
pub type Function<T> = Box<dyn FnMut() -> BFuture<T> + Send>;

#[derive(Clone, Debug)]
pub struct Scheduler<T>(Sender<(Duration, Function<T>)>);
impl<T> Scheduler<T> {
    pub fn schedule_task<
        Fut: Future<Output = (Option<Duration>, T)> + 'static + Send
    >(&self, duration: Duration, mut task: impl FnMut() -> Fut + Send + 'static) {
        self.0.send(
            (duration, Box::new(move || Box::pin(task()) as BFuture<T>) as Function<T>)
        ).unwrap();
    }
}

pub struct Thread<T: Send + 'static>(Receiver<T>, Sender<u8>);
impl<T: Send + 'static> Thread<T> {
    pub fn new(runtime: &tokio::runtime::Runtime) -> (Self, Scheduler<T>) {
        let (c_sender, results) = std::sync::mpsc::channel();
        let (sender, receiver) = std::sync::mpsc::channel();
        let (s_sender, s_receiver) = std::sync::mpsc::channel();
        runtime.spawn(Self::async_loop(receiver, s_receiver, c_sender));
        (
            Thread(results, s_sender),
            Scheduler(sender)
        )
    }

    pub fn results(&self) -> Vec<T> {self.0.try_iter().collect()}

    pub fn resume(&self) {self.1.send(0).unwrap();}
    pub fn pause(&self) {self.1.send(1).unwrap();}
    pub fn close(self) {self.1.send(2).unwrap();}

    async fn async_loop(rc: Receiver<(Duration, Function<T>)>, src: Receiver<u8>, tx: Sender<T>) {
        let time = Instant::now();
        let mut tasks = Vec::new();
        let mut paused = false;
        loop {
            if paused {
                match src.recv() {
                    Ok(0) => paused = false,
                    Ok(1) => {},
                    _ => return,
                };
            } else {
                match src.try_recv() {
                    Ok(0) => {},
                    Ok(1) => paused = false,
                    Err(TryRecvError::Empty) => {
                        while let Ok((duration, function)) = rc.try_recv() {
                            tasks.push((time.elapsed()+duration, function));
                        }
                        for (duration, mut function) in std::mem::take(&mut tasks) {
                            if duration < time.elapsed() {
                                let (duration, result) = function().await;
                                if tx.send(result).is_err() {return;};
                                if let Some(duration) = duration {
                                    tasks.push((time.elapsed()+duration, function));
                                }
                            } else {
                                tasks.push((duration, function));
                            }
                        }
                    },
                    _ => return
                }
            }
        }
    }
}

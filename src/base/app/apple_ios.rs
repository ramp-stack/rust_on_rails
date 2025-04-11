use crate::base::*;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use tokio::runtime::{Builder, Runtime};

const THREAD_TICK: Duration = Duration::from_millis(15);

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Snapshot(State);

pub struct BaseBackgroundApp<BA: BaseBackgroundAppTrait> {
    runtime: Runtime,
    ctx: BaseAsyncContext,
    app: BA,
}

impl<BA: BaseBackgroundAppTrait + 'static> BaseBackgroundApp<BA> {
    pub fn new(name: &str) -> Self {
        Logger::start(BA::LOG_LEVEL);
        let runtime = Builder::new_current_thread().build().unwrap();
        let mut ctx = BaseAsyncContext::new(name);
        let app = runtime.block_on(BA::new(&mut ctx));
        BaseBackgroundApp{runtime, ctx, app}
    }

    pub fn start(&mut self) {
        loop {
            self.runtime.block_on(self.app.on_tick(&mut self.ctx));
            std::thread::sleep(THREAD_TICK);
        }
    }
}

pub struct BaseApp<A: BaseAppTrait + 'static> {
    runtime: Runtime,
    async_thread: Thread<Callback>,
    async_ctx: BaseAsyncContext,
    ctx: BaseContext,
    app: A
}

impl<A: BaseAppTrait + 'static> BaseApp<A> {
    pub fn new(name: &'static str) -> Self {
        Logger::start(A::LOG_LEVEL);
        let runtime = Builder::new_multi_thread().worker_threads(1).build().unwrap();
        runtime.spawn(async {
            BaseBackgroundApp::<BA>::new(name).start();
            BA::new(&mut async_ctx);
            loop {
                
            }
        });
        let (async_thread, scheduler) = Thread::new(&runtime);
        scheduler.schedule_task(THREAD_TICK, || {
            async {
                let mut async_ctx = BaseAsyncContext::new(name);
                (Some(THREAD_TICK), A::on_async_tick(&mut async_ctx).await)
            }
        });

        let async_ctx = BaseAsyncContext::new(name);
        let state = runtime.block_on(async {Cache::new(name).get::<Snapshot>().await.0});

        let mut ctx = BaseContext{name: name.to_string(), state, scheduler};
        let app = runtime.block_on(A::new(&mut ctx));
        BaseApp{runtime, async_thread, async_ctx, ctx, app}
    }

    pub fn on_resume<W: HasWindowHandle + HasDisplayHandle>(
        &mut self, window: Arc<W>, width: u32, height: u32, scale_factor: f64
    ) {
        self.async_thread.resume();
        self.runtime.block_on(
            self.app.on_resume(&mut self.ctx, window, width, height, scale_factor)
        );
    }

    pub fn on_pause(&mut self) {
        self.async_thread.pause();
        self.app.on_pause(&mut self.ctx);
    }
    pub fn on_close(mut self) {
        self.async_thread.close();
        self.app.on_close(&mut self.ctx);
        self.runtime.block_on(self.async_ctx.cache.set(Snapshot(self.ctx.state)));
        self.runtime.shutdown_background();
    }
    pub fn on_tick(&mut self) {
        self.async_thread.results().into_iter().for_each(|s| s(&mut self.ctx.state));
        self.app.on_tick(&mut self.ctx);
    }
    pub fn on_event(&mut self, event: Event) {self.app.on_event(&mut self.ctx, event);}
}

#[macro_export]
macro_rules! create_base_entry_points {
    ($app:ty, $bg_app:ty) => {
        pub fn desktop_main() {
            if std::env::args().collect::<Vec<_>>().len() > 1 {
                BaseBackgroundApp::<$bg_app>::new(env!("CARGO_PKG_NAME")).start()
            } else {
                WinitApp::new(BaseApp::<$app>::new(env!("CARGO_PKG_NAME"))).start()
            }
        }
    };
}

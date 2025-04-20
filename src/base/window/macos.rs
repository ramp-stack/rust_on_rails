use super::WinitEventHandler;

use std::sync::Arc;

use crate::base::*;

use tokio::runtime::{Builder, Runtime};
//  use raw_window_handle::{HasWindowHandle, HasDisplayHandle};

use winit_crate::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit_crate::event::WindowEvent as WinitWindowEvent;
use winit_crate::application::ApplicationHandler;
use winit_crate::window::{Window, WindowId};



pub struct _BackgroundApp;
impl _BackgroundApp {
    pub fn start<BA: BackgroundApp + 'static>(name: &str) {
        Logger::start(BA::LOG_LEVEL);
        let runtime = Builder::new_current_thread().build().unwrap();
        let (param, tasks) = runtime.block_on(async {
            let mut ctx = AsyncContext{cache: Cache::new(name).await};
            let mut app = BA::new(&mut ctx).await;
            let tasks = app.register_tasks(&mut ctx).await;
            ((app, ctx), tasks)
        });
        runtime.block_on(Thread::new(param, tasks).0.async_loop())
    }
}

pub struct WindowApp<A: BaseAppTrait + 'static> {
    runtime: Runtime,
    window: Option<Arc<Window>>,
    event_handler: WinitEventHandler,
    thread_handle: ThreadHandle<Callback>,
    ctx: BaseContext,
    app: Option<A>,
}

impl<A: BaseAppTrait> BaseApp<A> {
    pub fn new(name: &'static str) -> Self {
        Logger::start(A::LOG_LEVEL);
        let runtime = Builder::new_multi_thread().worker_threads(1).build().unwrap();
        let (thread, thread_handle) = runtime.block_on(async {
            let tasks = A::register_tasks().await;
            let cache = Cache::new(name).await;
            let actx = AsyncContext{cache};
            let (thread, handle) = Thread::new(actx, tasks);
            (thread, handle)
        });
        runtime.spawn(thread.async_loop());
        BaseApp{
            runtime,
            window: None,
            event_handler: WinitEventHandler::default(),
            thread_handle,
            ctx: BaseContext{state: State::default()},
            app: None
        }
    }

    pub fn start(mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut self).unwrap();

        self.close()
    }

    fn close(mut self) {
        self.app.as_mut().unwrap().on_event(&mut self.ctx, WindowEvent::Close);
        self.thread_handle.close();
        self.runtime.shutdown_background();
    }

    fn window(&self) -> Arc<Window> {self.window.clone().unwrap()}
}

impl<A: BaseAppTrait + 'static> ApplicationHandler for BaseApp<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
         self.window = Some(Arc::new(event_loop.create_window(
            Window::default_attributes().with_title("orange")
        ).unwrap()));

        let size = self.window().inner_size();
        let scale_factor = self.window().scale_factor();
        let window = self.window.clone().unwrap();

        if self.app.is_none() {
            self.app = Some(self.runtime.block_on(A::new(
                &mut self.ctx, window, size.width, size.height, scale_factor
            )));
        } else {
            self.runtime.block_on(self.app.as_mut().unwrap().on_resume(
                &mut self.ctx, window, size.width, size.height, scale_factor
            ));
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.app.as_mut().unwrap().on_event(&mut self.ctx, WindowEvent::Pause);
        self.thread_handle.pause();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, i: WindowId, event: WinitWindowEvent) {
        self.window().request_redraw();
        if i == self.window().id() {
            match event {
                WinitWindowEvent::CloseRequested => {
                    event_loop.exit();
                },
                WinitWindowEvent::RedrawRequested => {
                    self.thread_handle.results().into_iter().for_each(|s| s(&mut self.ctx.state));
                    self.app.as_mut().unwrap().on_event(&mut self.ctx, WindowEvent::Tick);
                },
                WinitWindowEvent::Occluded(occluded) => {
                    if occluded {
                        self.app.as_mut().unwrap().on_event(&mut self.ctx, WindowEvent::Pause);
                        self.thread_handle.pause();
                    } else {
                        self.thread_handle.resume();
                        let window = self.window.clone().unwrap();
                        let size = self.window().inner_size();
                        let scale_factor = self.window().scale_factor();
                        self.runtime.block_on(self.app.as_mut().unwrap().on_resume(
                            &mut self.ctx, window, size.width, size.height, scale_factor
                        ));
                    }
                },
                event => {
                    if let Some(base_event) = self.event_handler.convert_event(event) {
                        self.app.as_mut().unwrap().on_event(&mut self.ctx, base_event);
                    }
                }
            }
        }
    }
}

#[macro_export]
macro_rules! create_app_entry_points {
    ($app:ty, $bg_app:ty) => {
        pub fn desktop_main() {
            if std::env::args().collect::<Vec<_>>().len() > 1 {
                _BackgroundApp::start::<$bg_app>(env!("CARGO_PKG_NAME"))
            } else {
                BaseApp::<$app>::new(env!("CARGO_PKG_NAME")).start()
            }
        }
    };
}

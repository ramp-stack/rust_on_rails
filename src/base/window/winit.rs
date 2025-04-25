use std::future::Future;
use std::path::PathBuf;
use std::sync::{Mutex, Arc};

use winit_crate::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit_crate::event::{ElementState, WindowEvent as WinitWindowEvent, TouchPhase, Touch};
use winit_crate::application::ApplicationHandler;
use winit_crate::window::{Window, WindowId};

use super::{WindowAppTrait, WindowEvent, MouseState, KeyboardState};

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

pub struct Winit<A: WindowAppTrait + 'static> {
    scale_factor: f64,
    future: Option<BlockingFuture<A>>,
    window: Option<Arc<Window>>,
    mouse: (u32, u32),
    size: (u32, u32),
    name: Option<PathBuf>,
    app: Option<A>
}

impl<A: WindowAppTrait + 'static> Winit<A> {
    pub fn new(name: PathBuf) -> Self {
        Winit{
            scale_factor: 0.0,
            future: None,
            window: None,
            mouse: (0, 0),
            size: (0, 0),
            name: Some(name),
            app: None
        }
    }

    #[cfg(target_os = "android")]
    pub fn start(mut self, app: AndroidApp) {
        let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut self).unwrap();
    }

    #[cfg(target_os = "ios")]
    pub fn start(mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut self).unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    pub fn start(mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).unwrap();
    }

    #[cfg(not(any(target_os = "android", target_os="ios", target_arch = "wasm32")))]
    pub fn start(mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut self).unwrap();
    }

    fn close(&mut self) {
        self.check_future();
        BlockingRuntime::block_on(self.app.take().unwrap().close());
    }

    fn window(&self) -> Arc<Window> {self.window.clone().unwrap()}

    fn check_future(&mut self) {
        if let Some(future) = self.future.take() {self.app = Some(future.unwrap());}
    }

    fn app_event(&mut self, event: WindowEvent<Arc<Window>>) {
        self.check_future();
        if self.app.is_none() {return;}//Already Closed
        let mut app = self.app.take().unwrap();
        self.future = Some(BlockingRuntime::block_on(async move {
            app.on_event(event).await;
            app
        }));
    }
}

impl<A: WindowAppTrait + 'static> ApplicationHandler for Winit<A> {
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window().request_redraw();
     }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
         self.window = Some(Arc::new(event_loop.create_window(
            Window::default_attributes().with_title("orange")
        ).unwrap()));

        let size = self.window().inner_size();
        self.size = size.into();
        let scale_factor = self.window().scale_factor();
        self.scale_factor = scale_factor;
        let window = self.window.clone().unwrap();
        if self.app.is_some() {
            self.app_event(WindowEvent::Resumed{
                window: window.clone(), width: size.width, height: size.height, scale_factor
            });
        } else {
            self.future = Some(BlockingRuntime::block_on(A::new(
                self.name.take().unwrap(), window, size.width, size.height, scale_factor
            )))
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.app_event(WindowEvent::Paused);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, i: WindowId, event: WinitWindowEvent) {
        self.window().request_redraw();
        if i == self.window().id() {
            match event {
                WinitWindowEvent::CloseRequested => {
                    self.close();
                    event_loop.exit();
                },
                WinitWindowEvent::RedrawRequested => {
                    self.app_event(WindowEvent::Tick);
                },
                WinitWindowEvent::Occluded(occluded) => {
                    if occluded {
                        self.app_event(WindowEvent::Paused);
                    } else {
                        let size = self.window().inner_size();
                        self.size = (size.width, size.height);
                        let window = self.window.clone().unwrap();
                        let scale_factor = self.window().scale_factor();
                        self.scale_factor = scale_factor;
                        self.app_event(WindowEvent::Resumed{
                            window: window.clone(), width: size.width, height: size.height, scale_factor
                        });
                    }
                },
                WinitWindowEvent::Resized(size) => {
                    self.size = size.into();
                    let scale_factor = self.scale_factor;
                    self.app_event(WindowEvent::Resized{
                        width: size.width, height: size.height, scale_factor
                    });
                },
                WinitWindowEvent::ScaleFactorChanged{scale_factor, ..} => {
                    let size = self.size;
                    self.scale_factor = scale_factor;
                    self.app_event(WindowEvent::Resized{
                        width: size.0, height: size.1, scale_factor
                    });
                },
                WinitWindowEvent::Touch(Touch{location, phase, ..}) => {
                    self.mouse = (location.x as u32, location.y as u32);
                    let position = self.mouse;
                    self.app_event(WindowEvent::Mouse{position, state: match phase {
                        TouchPhase::Started => MouseState::Pressed,
                        TouchPhase::Moved => MouseState::Moved,
                        TouchPhase::Ended => MouseState::Released,
                        TouchPhase::Cancelled => MouseState::Released
                    }});
                },
                WinitWindowEvent::CursorMoved{position, ..} => {
                    if self.mouse != (position.x as u32, position.y as u32) {
                        self.mouse = (position.x as u32, position.y as u32);
                        self.app_event(WindowEvent::Mouse{position: self.mouse, state: MouseState::Moved});
                    }
                },
                WinitWindowEvent::MouseInput{state, ..} => {
                    self.app_event(WindowEvent::Mouse{position: self.mouse, state: match state {
                        ElementState::Pressed => MouseState::Pressed,
                        ElementState::Released => MouseState::Released,
                    }});
                },
                WinitWindowEvent::KeyboardInput{event, ..} => {
                    self.app_event(WindowEvent::Keyboard{
                        key: event.logical_key, state: match event.state {
                        ElementState::Pressed => KeyboardState::Pressed,
                        ElementState::Released => KeyboardState::Released,
                    }});
                },
                _ => {}
            }
        }
    }
}

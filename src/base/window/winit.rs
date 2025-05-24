use std::future::Future;
use std::path::PathBuf;
use std::sync::{Mutex, Arc};
use std::time::{Duration, Instant};

use winit_crate::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit_crate::event::{ElementState, WindowEvent as WinitWindowEvent, TouchPhase, Touch, MouseScrollDelta};
use winit_crate::application::ApplicationHandler;
use winit_crate::window::{Window, WindowId};

#[cfg(target_os="android")]
use winit_crate::platform::android::activity::AndroidApp;
#[cfg(target_os="android")]
use winit_crate::platform::android::EventLoopBuilderExtAndroid;

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
            tokio::runtime::Builder::new_current_thread().enable_time().enable_io().build().unwrap().block_on(task)
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
    prev_touch: Option<(f64, f64)>,
    touch_start_time: Option<Instant>,
    mouse: (u32, u32, f32, f32), // x, y, mouse wheel threshold x, y
    size: (u32, u32),
    name: Option<PathBuf>,
    app: Option<A>,
}

impl<A: WindowAppTrait + 'static> Winit<A> {
    pub fn new(name: PathBuf) -> Self {
        Winit{
            scale_factor: 0.0,
            future: None,
            window: None,
            prev_touch: None,
            touch_start_time: None,
            mouse: (0, 0, 0.0, 0.0),
            size: (0, 0),
            name: Some(name),
            app: None,
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
        if self.window.is_some() {self.window().request_redraw();}
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
                WinitWindowEvent::Touch(Touch { location, phase, .. }) => {
                    let x = location.x;
                    let y = location.y;
                    let position = (x as u32, y as u32);
                
                    match phase {
                        // start a timer when the touch phase started
                        // when touch phase:: started, start a timer and app_event a MouseState::OnPress
                        // when touch phase:: Ended or Cannceled, stop the timer. If the timer is less than 1 second then app_event a MouseState::OnRelease and reset the timer
                        // if  the timer was greater than 1 second, then reset the timer, and emmit the app_event Released 
                        // TouchPhase::Started => {
                        //     self.prev_touch = Some((x, y));
                        //     self.app_event(WindowEvent::Mouse {
                        //         position,
                        //         state: MouseState::Pressed,
                        //     });
                        // }

                        // TouchPhase::Ended | TouchPhase::Cancelled => {
                        //     self.prev_touch = None;
                        //     self.mouse.2 = 0.0;
                        //     self.mouse.3 = 0.0;
                
                        //     self.app_event(WindowEvent::Mouse {
                        //         position,
                        //         state: MouseState::Released,
                        //     });
                        // }

                        TouchPhase::Started => {
                            self.prev_touch = Some((x, y));
                            self.touch_start_time = Some(Instant::now());

                            self.app_event(WindowEvent::Mouse {
                                position,
                                state: MouseState::Pressed,
                            });
                        }

                        TouchPhase::Ended | TouchPhase::Cancelled => {
                            self.prev_touch = None;
                            self.mouse.2 = 0.0;
                            self.mouse.3 = 0.0;

                            let held_for = self.touch_start_time
                                .take()
                                .map(|start| start.elapsed())
                                .unwrap_or_default();

                            if held_for < Duration::from_millis(200) {
                                // Short press
                                self.app_event(WindowEvent::Mouse {
                                    position,
                                    state: MouseState::Released,
                                });
                            } else {
                                // Long press release
                                self.app_event(WindowEvent::Mouse {
                                    position,
                                    state: MouseState::LongPressReleased,
                                });
                            }
                        }
                
                        TouchPhase::Moved => {
                            if let Some((prev_x, prev_y)) = self.prev_touch {
                                let dx = x - prev_x;
                                let dy = y - prev_y;
                        
                                let scroll_speed = 0.3; // Tune this to adjust sensitivity
                                let scroll_x = -(dx as f32) * scroll_speed;
                                let scroll_y = -(dy as f32) * scroll_speed;
                        
                                if scroll_x.abs() > 0.01 || scroll_y.abs() > 0.01 {
                                    self.app_event(WindowEvent::Mouse {
                                        position,
                                        state: MouseState::Scroll(scroll_x, scroll_y),
                                    });
                                }
                        
                                self.prev_touch = Some((x, y));
                            }
                        }
                    }
                },                
                WinitWindowEvent::CursorMoved{position, ..} => {
                    if self.mouse.0 != position.x as u32 && self.mouse.1 != position.y as u32 {
                        self.mouse.0 = position.x as u32;
                        self.mouse.1 = position.y as u32;
                        self.app_event(WindowEvent::Mouse{position: (self.mouse.0, self.mouse.1), state: MouseState::Moved});
                    }
                },
                WinitWindowEvent::MouseInput{state, ..} => {
                    self.app_event(WindowEvent::Mouse{position: (self.mouse.0, self.mouse.1), state: match state {
                        ElementState::Pressed => MouseState::Pressed,
                        ElementState::Released => MouseState::Released,
                    }});
                },
                WinitWindowEvent::MouseWheel{delta, phase, ..} => {
                    let position = (self.mouse.0, self.mouse.1);
                    if let TouchPhase::Moved = phase {
                        let pos = match delta {
                            MouseScrollDelta::LineDelta(x, y) => (x, y),
                            MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
                        };
                        let scroll_speed = 0.2; // Tune this to adjust sensitivity
                        self.mouse.2 += -pos.0 * scroll_speed;
                        self.mouse.3 += -pos.1 * scroll_speed;
                        self.app_event(WindowEvent::Mouse{position, state: MouseState::Scroll(self.mouse.2, self.mouse.3)});
                    }
                    if let TouchPhase::Ended = phase {
                        self.mouse.2 = 0.0;
                        self.mouse.3 = 0.0;
                    }
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

use wgpu_canvas::CanvasAtlas;

mod structs;
use structs::Size;
pub use structs::{Area, Color, CanvasItem, Shape, Text, Image, Font, Align};

use crate::base;
use crate::base::{BaseAppTrait, BaseContext, WindowHandle, AsyncTasks, State};

mod renderer;
use renderer::Canvas;

use std::future::Future;
use std::time::Instant;
use std::sync::Arc;

type Component = (wgpu_canvas::Area, wgpu_canvas::CanvasItem);

pub struct CanvasContext<'a> {
    atlas: &'a mut CanvasAtlas,
    base_ctx: &'a mut BaseContext,
    components: &'a mut Vec<Component>,
    size: Size,
}

impl<'a> CanvasContext<'a> {
    pub fn new(base_ctx: &'a mut BaseContext, components: &'a mut Vec<Component>, atlas: &'a mut CanvasAtlas, size: Size) -> Self {
        CanvasContext{base_ctx, components, atlas, size}
    }

    pub fn clear(&mut self, color: Color) {
        self.components.clear();
        self.components.push((
            Area((0.0, 0.0), None).into_inner(u16::MAX, &self.size),
            CanvasItem::Shape(
                Shape::Rectangle(0.0, self.size.logical()),
                color
            ).into_inner(&self.size)
        ));
    }

    pub fn draw(&mut self, area: Area, item: CanvasItem) {
        let z = u16::MAX-1-(self.components.len()) as u16;
        let area = area.into_inner(z, &self.size);
        self.components.push((area, item.into_inner(&self.size)));
    }

    pub fn state(&mut self) -> &mut State {&mut self.base_ctx.state}
}

pub trait CanvasAppTrait {

    fn register_tasks() -> impl Future<Output = AsyncTasks> where Self: Sized;
    fn new<'a>(ctx: &'a mut CanvasContext<'a>, width: f32, height: f32) -> impl std::future::Future<Output = Self> where Self: Sized;

    fn on_event<'a>(&'a mut self, ctx: &'a mut CanvasContext<'a>, event: WindowEvent);
}

pub struct CanvasApp<A: CanvasAppTrait> {
    components: Vec<Component>,
    canvas: Canvas,
    atlas: CanvasAtlas,
    size: Size,
    app: A,
    time: Instant
}


impl<A: CanvasAppTrait> CanvasApp<A> {
    fn tick(&mut self) {
        let items = self.components.drain(..).collect::<Vec<_>>();
        self.canvas.prepare(&mut self.atlas, items);
        self.canvas.render();
        log::error!("last_frame: {:?}", self.time.elapsed());
        self.time = Instant::now();
    }
}

impl<A: CanvasAppTrait> BaseAppTrait for CanvasApp<A> {
    const LOG_LEVEL: log::Level = log::Level::Error;

    async fn new<W: WindowHandle>(ctx: &mut BaseContext, window: Arc<W>, width: u32, height: u32, scale_factor: f64) -> Self {
        let mut canvas = Canvas::new(window).await;
        let (width, height) = canvas.resize(width, height);
        let mut components = Vec::new();
        let mut atlas = CanvasAtlas::default();
        let size = Size::new(width as f32, height as f32, scale_factor);
        let mut context = CanvasContext::new(ctx, &mut components, &mut atlas, size);
        let app = A::new(&mut context, width as f32, height as f32).await;
        CanvasApp{
            components,
            canvas,
            atlas,
            size,
            app,
            time: Instant::now()
        }
    }

    async fn register_tasks() -> AsyncTasks {A::register_tasks().await}

    async fn on_resume<W: WindowHandle>(
        &mut self, ctx: &mut BaseContext, window: Arc<W>, width: u32, height: u32, scale_factor: f64
    ) {
        self.canvas.resume(window);
        let (width, height) = self.canvas.resize(width, height);
        self.size = Size::new(width as f32, height as f32, scale_factor);
        let mut context = CanvasContext::new(ctx, &mut self.components, &mut self.atlas, self.size);
        let event = WindowEvent::Resume{width: self.size.logical().0, height: self.size.logical().1};
        self.app.on_event(&mut context, event);
    }

    fn on_event(&mut self, ctx: &mut BaseContext, event: base::WindowEvent) {
        let mut tick = false;
        let event = match event {
            base::WindowEvent::Resize{width, height, scale_factor} => {
                let (width, height) = self.canvas.resize(width, height);
                self.size = Size::new(width as f32, height as f32, scale_factor);
                WindowEvent::Resize{width: self.size.logical().0, height: self.size.logical().1}
            },
            base::WindowEvent::Mouse{position, state} => WindowEvent::Mouse{position: (
                self.size.scale_logical(position.0 as f32),
                self.size.scale_logical(position.1 as f32)
            ), state},
            base::WindowEvent::Keyboard{key, state} => WindowEvent::Keyboard{key, state},
            base::WindowEvent::Tick => {tick = true; WindowEvent::Tick},
            base::WindowEvent::Pause => WindowEvent::Pause,
            base::WindowEvent::Close => WindowEvent::Close,
        };
        let mut context = CanvasContext::new(ctx, &mut self.components, &mut self.atlas, self.size);
        self.app.on_event(&mut context, event);

        if tick {self.tick()}//CanvasTick must happen after App tick
    }
}

pub use crate::base::{MouseState, KeyboardState, NamedKey, SmolStr, Key};

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    Resize{width: f32, height: f32},
    Mouse{position: (f32, f32), state: MouseState},
    Keyboard{key: Key, state: KeyboardState},
    Resume{width: f32, height: f32},
    Pause,
    Close,
    Tick
}

#[macro_export]
macro_rules! create_canvas_entry_points {
    ($app:ty, $bg_app:ty) => {
        create_base_entry_points!(CanvasApp::<$app>, $bg_app);
    };
}

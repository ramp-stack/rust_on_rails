use crate::base;
use crate::base::driver::state::State;
use crate::base::driver::runtime::Tasks;
use crate::base::driver::camera::Camera;
use crate::base::BaseAppTrait;

use crate::base::renderer::wgpu_canvas as canvas;

use std::future::Future;
use std::time::Instant;

pub use canvas::{Canvas, CanvasItem, Area, Image, Text, Font, Shape, Color, Event, Span, Align};
pub use canvas::{MouseState, KeyboardState, NamedKey, SmolStr, Key, Cursor};
pub use crate::base::HeadlessContext;

pub trait App {
    fn background_tasks(ctx: &mut HeadlessContext) -> impl Future<Output = Tasks>;
    fn new(ctx: &mut Context<'_>) -> impl Future<Output = (Self, Tasks)> where Self: Sized;
    fn on_event(&mut self, ctx: &mut Context<'_>, event: Event);
}

pub struct Context<'a> {//CuttingContext
    size: (f32, f32),
    base_context: &'a mut base::Context<'a, Canvas>,
}

impl AsMut<canvas::Context> for Context<'_> {
    fn as_mut(&mut self) -> &mut canvas::Context {
        self.base_context.as_mut()
    }
}

impl<'a> Context<'a> {
    fn new(
        size: (f32, f32),
        base_context: &'a mut base::Context<'a, Canvas>
    ) -> Self {Context{size, base_context}}

    pub fn clear(&mut self, color: Color) {self.base_context.as_mut().clear(color);}
    pub fn draw(&mut self, area: Area, item: CanvasItem) {self.base_context.as_mut().draw(area, item);}
    pub fn add_font(&mut self, font: &[u8]) -> Font {self.base_context.as_mut().add_font(font)}
    pub fn add_image(&mut self, image: image::RgbaImage) -> Image {self.base_context.as_mut().add_image(image)}

    pub fn size(&self) -> (f32, f32) {self.size}
    pub fn state(&mut self) -> &mut State {self.base_context.state()}


    pub fn open_camera() -> Camera { Camera::new() }
}

pub struct CanvasApp<A: App> {
    size: (f32, f32),
    app: A,

    time: Instant
}

impl<A: App> BaseAppTrait<Canvas> for CanvasApp<A> {
    const LOG_LEVEL: log::Level = log::Level::Error;

    async fn background_tasks(ctx: &mut HeadlessContext) -> Tasks {A::background_tasks(ctx).await}

    async fn new<'a>(base_context: &'a mut base::Context<'a, Canvas>, _ctx: &mut HeadlessContext, width: f32, height: f32) -> (Self, Tasks) {
        let size = (width, height);
        let mut ctx = Context::new(size, base_context);
        let (app, tasks) = A::new(&mut ctx).await;
        (CanvasApp{
            size,
            app,

            time: Instant::now()
        }, tasks)
    }

    fn on_event<'a>(&'a mut self, base_context: &'a mut base::Context<'a, Canvas>, event: Event) {
        match &event {
            Event::Tick => {
                log::error!("last_frame: {:?}", self.time.elapsed());
                self.time = Instant::now();
            },
            Event::Resumed{width, height} | Event::Resized{width, height} => {self.size = (*width, *height);},
            _ => {}
        };
        let mut ctx = Context::new(self.size, base_context);
        self.app.on_event(&mut ctx, event);
    }
    async fn close(self) {}
}

#[macro_export]
macro_rules! create_entry_points {
    ($app:ty) => {
        create_base_entry_points!(Canvas, CanvasApp<$app>);
    };
}

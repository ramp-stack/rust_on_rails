<<<<<<< HEAD
pub use crate::base::renderer::wgpu_canvas::{
    Canvas, CanvasContext, Area, Color, CanvasItem, Shape, Text, Image, Font
};
=======
use wgpu_canvas::CanvasAtlas;

mod structs;
use structs::Size;
pub use structs::{Area, Color, CanvasItem, Shape, Text, Image, Font, Align};
>>>>>>> 2a325e10b5b30a820c368e3e9b0bb05eb8cc45b9

use crate::base;
use base::driver::state::State;
use base::BaseAppTrait;

use std::future::Future;
use std::time::Instant;

use crate::base::runtime::Tasks;

pub use crate::base::{HeadlessContext, Event, MouseState, KeyboardState, NamedKey, SmolStr, Key};

pub trait App {
    fn background_tasks(ctx: &mut HeadlessContext) -> impl Future<Output = Tasks>;
    fn new(ctx: &mut Context<'_>) -> impl Future<Output = (Self, Tasks)> where Self: Sized;
    fn on_event(&mut self, ctx: &mut Context<'_>, event: Event);
}

pub struct Context<'a> {
    size: (f32, f32),
    components: &'a mut Vec<(Area, CanvasItem)>,
    base_context: &'a mut base::Context<Canvas>,
}

impl AsMut<CanvasContext> for Context<'_> {
    fn as_mut(&mut self) -> &mut CanvasContext {
        self.base_context.render_ctx()
    }
}

impl<'a> Context<'a> {
    fn new(
        size: (f32, f32),
        components: &'a mut Vec<(Area, CanvasItem)>,
        base_context: &'a mut base::Context<Canvas>
    ) -> Self {Context{size, components, base_context}}

    pub fn clear(&mut self, color: Color) {
        self.components.clear();
        self.components.push((Area((0.0, 0.0), None), CanvasItem::Shape(Shape::Rectangle(0.0, self.size), color)));
    }

    pub fn draw(&mut self, area: Area, item: CanvasItem) {
        self.components.push((area, item));
    }

    pub fn size(&self) -> (f32, f32) {self.size}

    pub fn state(&mut self) -> &mut State {self.base_context.state()}

    pub fn add_font(&mut self, font: &[u8]) -> Font {self.base_context.render_ctx().add_font(font)}
    pub fn add_image(&mut self, image: image::RgbaImage) -> Image {self.base_context.render_ctx().add_image(image)}
}

pub struct CanvasApp<A: App> {
    components: Vec<(Area, CanvasItem)>,
    size: (f32, f32),
    app: A,

    time: Instant
}

impl<A: App> BaseAppTrait<Canvas> for CanvasApp<A> {
    const LOG_LEVEL: log::Level = log::Level::Error;

    async fn background_tasks(ctx: &mut HeadlessContext) -> Tasks {A::background_tasks(ctx).await}

    async fn new(base_context: &mut base::Context<Canvas>, _ctx: &mut HeadlessContext, width: f32, height: f32) -> (Self, Tasks) {
        let size = (width, height);
        let mut components = Vec::new();
        let mut ctx = Context::new(size, &mut components, base_context);
        let (app, tasks) = A::new(&mut ctx).await;
        (CanvasApp{
            components,
            size,
            app,

            time: Instant::now()
        }, tasks)
    }

    fn on_event(&mut self, base_context: &mut base::Context<Canvas>, event: Event) {
        match &event {
            Event::Resumed{width, height} | Event::Resized{width, height} => {self.size = (*width, *height);},
            _ => {}
        };
        let mut ctx = Context::new(self.size, &mut self.components, base_context);
        self.app.on_event(&mut ctx, event);
        
    }
    fn draw(&mut self, _: &mut base::Context<Canvas>) -> Vec<(Area, CanvasItem)> {
        log::error!("last_frame: {:?}", self.time.elapsed());
        self.time = Instant::now();
        self.components.clone()
    }
    async fn close(self) {}
}

#[macro_export]
macro_rules! create_entry_points {
    ($app:ty) => {
        create_base_entry_points!(Canvas, CanvasApp<$app>);
    };
}

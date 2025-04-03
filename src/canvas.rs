use wgpu_canvas::CanvasAtlas;
use crate::{WinitAppTrait, winit::WinitWindow};
pub use crate::winit::{MouseEvent, MouseState, KeyboardEvent, KeyboardState};

use std::time::Instant;

mod structs;
pub use structs::{Area, Color, CanvasItem, Shape, Text, Image, Font};
use structs::Size;

mod renderer;
use renderer::Canvas;

#[derive(Default)]
pub struct CanvasContext{
    components: Vec<(wgpu_canvas::Area, wgpu_canvas::CanvasItem)>,
    atlas: CanvasAtlas,
    size: Size,
}

impl CanvasContext {
    pub fn clear(&mut self, color: Color) {
        self.components.clear();
        self.components.push((
            Area((0, 0), None).into_inner(u16::MAX, &self.size),
            CanvasItem::Shape(
                Shape::Rectangle(0, self.size.logical()),
                color
            ).into_inner(&self.size)
        ));
    }

    pub fn draw(&mut self, area: Area, item: CanvasItem) {
        let z = u16::MAX-1-(self.components.len()) as u16;
        let area = area.into_inner(z, &self.size);
        self.components.push((area, item.into_inner(&self.size)));
    }
}

pub trait CanvasAppTrait {
    fn new(ctx: &mut CanvasContext, width: u32, height: u32) -> impl std::future::Future<Output = Self> where Self: Sized;

    fn on_resize(&mut self, ctx: &mut CanvasContext, width: u32, height: u32) -> impl std::future::Future<Output = ()>;
    fn on_tick(&mut self, ctx: &mut CanvasContext) -> impl std::future::Future<Output = ()>;
    fn on_mouse(&mut self, ctx: &mut CanvasContext, event: MouseEvent) -> impl std::future::Future<Output = ()>;
    fn on_keyboard(&mut self, ctx: &mut CanvasContext, event: KeyboardEvent) -> impl std::future::Future<Output = ()>;
}

pub struct CanvasApp<A: CanvasAppTrait> {
    context: CanvasContext,
    canvas: Canvas,
    app: A,
    time: Instant
}

impl<A: CanvasAppTrait> WinitAppTrait for CanvasApp<A> {
    async fn new(window: WinitWindow, width: u32, height: u32, scale_factor: f64) -> Self {
        let mut canvas = Canvas::new(window).await;
        let (width, height) = canvas.resize(width, height);
        let mut context = CanvasContext{
            size: Size::new(width, height, scale_factor),
            ..Default::default()
        };
        let app = A::new(&mut context, width, height).await;

        CanvasApp{
            context,
            canvas,
            app,
            time: Instant::now()
        }
    }

    async fn on_resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        let (width, height) = self.canvas.resize(width, height);
        let size = Size::new(width, height, scale_factor);
        self.app.on_resize(&mut self.context, size.logical().0, size.logical().1).await;
        self.context.size = size;
    }

    async fn prepare(&mut self) {
        self.app.on_tick(&mut self.context).await;
        let items = self.context.components.drain(..).collect();

        self.canvas.prepare(&mut self.context.atlas, items);
    }

    async fn render(&mut self) {
        self.canvas.render();
        log::error!("last_frame: {}", self.time.elapsed().as_nanos());
        self.time = Instant::now();
    }

    async fn on_mouse(&mut self, mut event: MouseEvent) {
        event.position = (
            self.context.size.scale_logical(event.position.0),
            self.context.size.scale_logical(event.position.1)
        );
        self.app.on_mouse(&mut self.context, event).await
    }
    async fn on_keyboard(&mut self, event: KeyboardEvent) {
        self.app.on_keyboard(&mut self.context, event).await
    }
}

#[macro_export]
macro_rules! create_canvas_entry_points {
    ($app:ty) => {
        create_winit_entry_points!(CanvasApp::<$app>);
    };
}

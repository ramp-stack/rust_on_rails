use super::{WinitAppTrait, winit::WinitWindow};

use image::{GenericImage, RgbaImage};

use wgpu_canvas::CanvasAtlas;
pub use wgpu_canvas::{Font, Image};

use std::time::Instant;

mod structs;
pub use structs::{Area, CanvasItem, Shape};
use structs::Size;

mod renderer;
use renderer::Canvas;

#[derive(Default)]
pub struct CanvasContext{
    components: Vec<(wgpu_canvas::Area, wgpu_canvas::CanvasItem)>,
    atlas: CanvasAtlas,
    size: Size,
    pub position: (u32, u32),
}

impl CanvasContext {

    pub fn new_font(&mut self, font: Vec<u8>) -> Font {
        Font::new(&mut self.atlas, font)
    }
    pub fn new_image(&mut self, image: RgbaImage) -> Image {
        Image::new(&mut self.atlas, image)
    }

    //pub fn messure_text(&mut self, t: &Text) -> (u32, u32) {self.atlas.messure_text(&t.into_inner(self.size))}

    pub fn width(&self) -> u32 {self.size.logical().0}
    pub fn height(&self) -> u32 {self.size.logical().1}

    pub fn clear(&mut self, color: &'static str) {
        self.components.clear();
      //self.components.push(
      //    CanvasItem::Shape(
      //        Area((0, 0), None),
      //        Shape::Rectangle(0, self.size.logical()),
      //        color, 255
      //    ).into_inner(u16::MAX, self.size)
      //);
    }

    pub fn draw(&mut self, area: Area, item: CanvasItem) {
        let z = u16::MAX-1-(self.components.len()) as u16;
        let area = area.into_inner(z, self.size);
        self.components.push((area, item.0));
    }
}

pub trait CanvasAppTrait {
    fn new(ctx: &mut CanvasContext) -> impl std::future::Future<Output = Self> where Self: Sized;
    fn draw(&mut self, ctx: &mut CanvasContext) -> impl std::future::Future<Output = ()>;

    fn on_click(&mut self, ctx: &mut CanvasContext) -> impl std::future::Future<Output = ()>;
    fn on_move(&mut self, ctx: &mut CanvasContext) -> impl std::future::Future<Output = ()>;
    fn on_press(&mut self, ctx: &mut CanvasContext, t: String) -> impl std::future::Future<Output = ()>;
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
        let mut context = CanvasContext::default();
        context.size = Size::new(width, height, scale_factor);
        let app = A::new(&mut context).await;

        CanvasApp{
            context,
            canvas,
            app,
            time: Instant::now()
        }
    }

    async fn prepare(&mut self, width: u32, height: u32, scale_factor: f64) {
        let (width, height) = self.canvas.resize(width, height);
        self.context.size = Size::new(width, height, scale_factor);

        self.app.draw(&mut self.context).await;
        let items = self.context.components.drain(..).collect();

        self.canvas.prepare(&mut self.context.atlas, items);
    }

    async fn render(&mut self) {
        log::error!("last_frame: {}", self.time.elapsed().as_millis());
        self.time = Instant::now();
        self.canvas.render();
    }

    async fn on_click(&mut self) {
        self.app.on_click(&mut self.context).await
    }
    async fn on_move(&mut self, x: u32, y: u32) {
        self.context.position = self.context.size.to_logical(x, y);
        self.app.on_move(&mut self.context).await
    }
    async fn on_press(&mut self, t: String) {
        self.app.on_press(&mut self.context, t).await
    }
}

#[macro_export]
macro_rules! create_canvas_entry_points {
    ($app:ty) => {
        create_winit_entry_points!(CanvasApp::<$app>);
    };
}

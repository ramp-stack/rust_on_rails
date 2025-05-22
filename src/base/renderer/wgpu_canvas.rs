use wgpu_canvas::{ImageAtlas, FontAtlas};

use super::{Renderer, RenderAppTrait, HasLifeEvents};
use crate::base::window::{WindowHandle, WindowEvent};

pub use wgpu_canvas::{Shape, Color, Area, Text, Span, Cursor, CursorAction, Align, Font, Image};
pub use crate::base::window::{MouseState, KeyboardState, NamedKey, SmolStr, Key};

#[derive(Debug, Clone, Copy)]
pub struct Scale(f64);
impl Scale {
    pub fn physical(&self, x: f32) -> f32 {
        (x as f64 * self.0) as f32
    }

    pub fn logical(&self, x: f32) -> f32 {
        (x as f64 / self.0) as f32
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Resized{width: f32, height: f32},
    Mouse{position: (f32, f32), state: MouseState},
    Keyboard{key: Key, state: KeyboardState},
    Resumed{width: f32, height: f32},
    Paused,
    Tick
}

impl HasLifeEvents for Event {
    fn is_resumed(&self) -> bool {matches!(self, Event::Resumed{..})}
    fn is_paused(&self) -> bool {matches!(self, Event::Paused)}
}

pub struct Context{
    scale: Scale,
    image: ImageAtlas,
    font: FontAtlas,
    components: Vec<(Area, wgpu_canvas::CanvasItem)>,
    size: (f32, f32)
}
impl Context {
    pub fn add_font(&mut self, font: &[u8]) -> Font {self.font.add(font)}
    pub fn add_image(&mut self, image: image::RgbaImage) -> Image {self.image.add(image)}
    pub fn add_svg(&mut self, svg: &[u8], scale: f32) -> Image {
        let svg = std::str::from_utf8(svg).unwrap();
        let svg = nsvg::parse_str(svg, nsvg::Units::Pixel, 96.0).unwrap();
        let rgba = svg.rasterize(scale).unwrap();
        let size = rgba.dimensions();
        self.image.add(image::RgbaImage::from_raw(size.0, size.1, rgba.into_raw()).unwrap())
    }
    pub fn size(&self) -> (f32, f32) {self.size}
    pub fn draw(&mut self, area: Area, item: CanvasItem) {
        let area = Area(
            (self.scale.physical(area.0.0), self.scale.physical(area.0.1)),
            area.1.map(|(x, y, w, h)| (
                self.scale.physical(x), self.scale.physical(y),
                self.scale.physical(w), self.scale.physical(h)
            ))
        );
        self.components.push((area, item.scale(&self.scale)));
    }

    pub fn clear(&mut self, color: Color) {
        self.components.clear();
        self.components.push((Area((0.0, 0.0), None),
            wgpu_canvas::CanvasItem::Shape(Shape::Rectangle(0.0,
                (self.scale.physical(self.size.0), self.scale.physical(self.size.1))
            ), color)
        ));
    }
}
impl AsMut<FontAtlas> for Context {fn as_mut(&mut self) -> &mut FontAtlas {&mut self.font}}
impl AsMut<ImageAtlas> for Context {fn as_mut(&mut self) -> &mut ImageAtlas {&mut self.image}}

#[derive(Clone, Debug)]
pub enum CanvasItem {
    Shape(Shape, Color),
    Image(Shape, Image, Option<Color>),
    Text(Text),
}

impl CanvasItem {
    fn scale(self, scale: &Scale) -> wgpu_canvas::CanvasItem {
        match self {
            CanvasItem::Shape(shape, color) => wgpu_canvas::CanvasItem::Shape(
                Self::scale_shape(shape, scale), color
            ),
            CanvasItem::Image(shape, image, color) => wgpu_canvas::CanvasItem::Image(
                Self::scale_shape(shape, scale), image, color
            ),
            CanvasItem::Text(text) => wgpu_canvas::CanvasItem::Text(Self::scale_text(text, scale))
        }
    }

    fn scale_text(text: Text, scale: &Scale) -> Text {
        Text::new(
            text.spans.into_iter().map(|s|
                Span::new(s.text, scale.physical(s.font_size), scale.physical(s.line_height), s.font, s.color)
            ).collect(),
            text.width.map(|w| scale.physical(w)),
            text.align,
            text.cursor,
        )
    }

    fn scale_shape(shape: Shape, scale: &Scale) -> Shape {
        match shape {
            Shape::Ellipse(s, size) => Shape::Ellipse(scale.physical(s), Self::scale_size(size, scale)),
            Shape::Rectangle(s, size) => Shape::Rectangle(scale.physical(s), Self::scale_size(size, scale)),
            Shape::RoundedRectangle(s, size, r) => Shape::RoundedRectangle(
                scale.physical(s), Self::scale_size(size, scale), scale.physical(r)
            ),
        }
    }

    fn scale_size(size: (f32, f32), scale: &Scale) -> (f32, f32) {
        (scale.physical(size.0), scale.physical(size.1))
    }
}

mod wgpu;
pub use wgpu::Canvas;

impl Renderer for Canvas {
    type Context = Context;
    type Event = Event;

    async fn new<W: WindowHandle + 'static>(
        window: W, width: u32, height: u32, scale_factor: f64
    ) -> (Self, Self::Context, (f32, f32)) {
        let (canvas, size) = Self::inner_new(window, width, height).await;
        let scale = Scale(scale_factor);
        let size = (scale.logical(size.0 as f32), scale.logical(size.1 as f32));
        let ctx = Context{scale, image: ImageAtlas::default(), font: FontAtlas::default(), components: Vec::new(), size};
        (canvas, ctx, size)
    }
        
    async fn on_event<W: WindowHandle, A: RenderAppTrait<Self>>(
        &mut self, app: &mut A, event: WindowEvent<W>
    ) {
        let ctx = app.ctx();
        let draw =  matches!(event, WindowEvent::Tick);
        let r_event = match event {
            WindowEvent::Resized{width, height, scale_factor} => {
                ctx.scale.0 = scale_factor;
                let size = self.resize::<W>(None, width, height);
                let size = (ctx.scale.logical(size.0 as f32), ctx.scale.logical(size.1 as f32));
                ctx.size = size;
                Event::Resized{width: size.0, height: size.1}
            },
            WindowEvent::Mouse{position, state} => {
                Event::Mouse{position: (
                    ctx.scale.logical(position.0 as f32), ctx.scale.logical(position.1 as f32)
                ), state}
            }
            WindowEvent::Keyboard{key, state} => Event::Keyboard{key, state},
            WindowEvent::Resumed{window, width, height, scale_factor} => {
                ctx.scale.0 = scale_factor;
                let size = self.resize(Some(window.into()), width, height);
                let size = (ctx.scale.logical(size.0 as f32), ctx.scale.logical(size.1 as f32));
                ctx.size = size;
                Event::Resumed{width: size.0, height: size.1}
            },
            WindowEvent::Paused => Event::Paused,
            WindowEvent::Tick => Event::Tick
        };
        app.on_event(r_event).await;
        let ctx = app.ctx();
        if draw {self.draw(&mut ctx.image, &mut ctx.font, ctx.components.drain(..).collect::<Vec<_>>());}
    }

    async fn close(self, _ctx: Self::Context) {}
}

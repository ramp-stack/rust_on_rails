use super::CanvasContext;

#[derive(Clone, Copy, Debug, Default)]
pub struct Size {
    width: u32,
    height: u32,
    scale_factor: f64
}

impl Size {
    pub fn new(width: u32, height: u32, scale_factor: f64) -> Self {
        Size{width, height, scale_factor}
    }

    pub fn scale_physical(&self, x: u32) -> u32 {
        (x as f64 * self.scale_factor).round() as u32
    }

    pub fn scale_logical(&self, x: u32) -> u32 {
        (x as f64 / self.scale_factor).floor() as u32
    }

    pub fn dscale_physical(&self, x: i32) -> i32 {
        (x as f64 * self.scale_factor).round() as i32
    }

  //pub fn dscale_logical(&self, x: i32) -> i32 {
  //    (x as f64 / self.scale_factor).floor() as i32
  //}

    pub fn logical(&self) -> (u32, u32) {
        (self.scale_logical(self.width), self.scale_logical(self.height))
    }

    pub fn physical(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    pub fn from_hex(color: &'static str, alpha: u8) -> Self {
        let ce = "Color was not a Hex Value";
        let c = hex::decode(color.strip_prefix('#').unwrap_or(color)).expect(ce);
        Color(c[0], c[1], c[2], alpha)
    }

    fn into_inner(self) -> wgpu_canvas::Color {
        wgpu_canvas::Color(self.0, self.1, self.2, self.3)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Area(pub (i32, i32), pub Option<(i32, i32, u32, u32)>);

impl Area {
    pub(crate) fn into_inner(self, z_index: u16, size: &Size) -> wgpu_canvas::Area {
        let psize = size.physical();
        let bounds = self.1.map(|(x, y, w, h)| {
            (size.dscale_physical(x), size.dscale_physical(y),
             size.scale_physical(w), size.scale_physical(h))
        }).unwrap_or((0, 0, psize.0, psize.1));
        wgpu_canvas::Area{
            z_index,
            bounds,
            offset: (size.dscale_physical(self.0.0), size.dscale_physical(self.0.1)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Ellipse(u32, (u32, u32)),
    Rectangle(u32, (u32, u32)),
    RoundedRectangle(u32, (u32, u32), u32),
}

impl Shape {
    pub(crate) fn into_inner(self, size: &Size) -> wgpu_canvas::Shape {
        let p = |s: (u32, u32)| (size.scale_physical(s.0), size.scale_physical(s.1));
        match self {
            Shape::Ellipse(stroke, s) => wgpu_canvas::Shape::Ellipse(
                size.scale_physical(stroke), p(s)
            ),
            Shape::Rectangle(stroke, s) => wgpu_canvas::Shape::Rectangle(
                size.scale_physical(stroke), p(s)
            ),
            Shape::RoundedRectangle(stroke, s, corner_radius) => {
                let corner_radius = size.scale_physical(corner_radius);
                wgpu_canvas::Shape::RoundedRectangle(
                    size.scale_physical(stroke), p(s), corner_radius
                )
            }
        }
    }

    pub fn size(&self) -> (u32, u32) {
        match self {
            Shape::Ellipse(_, size) => *size,
            Shape::Rectangle(_, size) => *size,
            Shape::RoundedRectangle(_, size, _) => *size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    pub text: String,
    pub color: Color,
    pub width: Option<u32>,
    pub size: u32,
    pub line_height: u32,
    pub font: Font
}

impl Text {
    pub fn new(
        text: &str,
        color: Color,
        width: Option<u32>,
        size: u32,
        line_height: u32,
        font: Font
    ) -> Self {
        Text{text: text.to_string(), color, width, size, line_height, font}
    }

    pub(crate) fn into_inner(self, size: &Size) -> wgpu_canvas::Text {
        wgpu_canvas::Text{
            text: self.text,
            color: self.color.into_inner(),
            width: self.width.map(|w| size.scale_physical(w)),
            size: size.scale_physical(self.size),
            line_height: size.scale_physical(self.line_height),
            font: self.font.into_inner()
        }
    }

    pub fn size(&self, ctx: &mut CanvasContext) -> (u32, u32) {
        let size = self.clone().into_inner(&ctx.size).size(ctx.atlas);
        (ctx.size.scale_logical(size.0), ctx.size.scale_logical(size.1))
    }
}

#[derive(Debug, Clone)]
pub enum CanvasItem {
    Shape(Shape, Color),
    Image(Shape, Image, Option<Color>),
    Text(Text)
}

impl CanvasItem {
    pub(crate) fn into_inner(self, size: &Size) -> wgpu_canvas::CanvasItem {
        match self {
            CanvasItem::Shape(shape, color) => {
                wgpu_canvas::CanvasItem::Shape(
                    shape.into_inner(size), color.into_inner()
                )
            },
            CanvasItem::Image(shape, image, color) => {
                wgpu_canvas::CanvasItem::Image(shape.into_inner(size), image.into_inner(),
                    color.map(|c| c.into_inner())
                )
            },
            CanvasItem::Text(text) => {
                wgpu_canvas::CanvasItem::Text(text.into_inner(size))
            }
        }
    }

    pub fn size(&self, ctx: &mut CanvasContext) -> (u32, u32) {
        match self {
            CanvasItem::Shape(shape, _) => shape.size(),
            CanvasItem::Image(shape, _, _) => shape.size(),
            CanvasItem::Text(text) => text.size(ctx),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image(wgpu_canvas::Image);
impl Image {
    pub fn new(ctx: &mut CanvasContext, image: image::RgbaImage) -> Self {
        Image(wgpu_canvas::Image::new(ctx.atlas, image))
    }

    pub fn svg(ctx: &mut CanvasContext, svg: &[u8], scale: f32) -> Self {
        let svg = std::str::from_utf8(svg).unwrap();
        let svg = nsvg::parse_str(svg, nsvg::Units::Pixel, 96.0).unwrap();
        let rgba = svg.rasterize(scale).unwrap();
        let size = rgba.dimensions();
        Image::new(ctx, image::RgbaImage::from_raw(size.0, size.1, rgba.into_raw()).unwrap())
    }

    pub(crate) fn into_inner(self) -> wgpu_canvas::Image {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Font(wgpu_canvas::Font);
impl Font {
    pub fn new(ctx: &mut CanvasContext, font: Vec<u8>) -> Self {
        Font(wgpu_canvas::Font::new(ctx.atlas, font))
    }

    pub(crate) fn into_inner(self) -> wgpu_canvas::Font {
        self.0
    }
}

use super::{Image, Font};

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
    pub fn new_logical(&self, px: u32, py: u32) -> (u32, u32) {
        (
            (px.min(self.width) as f64 / self.scale_factor).floor() as u32,
            (py.min(self.height) as f64 / self.scale_factor).floor() as u32,
        )
    }

    pub fn scale_physical(&self, x: u32) -> u32 {
        (x as f64 * self.scale_factor).round() as u32
    }

    pub fn new_physical(&self, lx: u32, ly: u32) -> (u32, u32) {
        (
            ((lx as f64 * self.scale_factor).round() as u32).min(self.width),
            ((ly as f64 * self.scale_factor).round() as u32).min(self.height),
        )
    }

    pub fn logical(&self) -> (u32, u32) {
        self.new_logical(self.width, self.height)
    }

    pub fn physical(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Area(pub (u32, u32), pub Option<(u32, u32, u32, u32)>);

impl Area {
    pub(crate) fn into_inner(self, z_index: u16, size: &Size) -> wgpu_canvas::Area {
        let psize = size.physical();
        let bounds = self.1.map(|(x, y, w, h)| {
            let xy = size.new_physical(x, y);
            let wh = size.new_physical(w, h);
            (xy.0, xy.1, wh.0, wh.1)
        }).unwrap_or((0, 0, psize.0, psize.1));
        wgpu_canvas::Area{
            z_index,
            bounds,
            offset: size.new_physical(self.0.0, self.0.1),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ShapeType {
    Ellipse(u32, (u32, u32)),
    Rectangle(u32, (u32, u32)),
    RoundedRectangle(u32, (u32, u32), u32),
}

impl ShapeType {
    pub(crate) fn into_inner(self, size: &Size) -> wgpu_canvas::Shape {
        let p = |s: (u32, u32)| size.new_physical(s.0, s.1);
        match self {
            ShapeType::Ellipse(stroke, s) => wgpu_canvas::Shape::Ellipse(
                size.scale_physical(stroke), p(s)
            ),
            ShapeType::Rectangle(stroke, s) => wgpu_canvas::Shape::Rectangle(
                size.scale_physical(stroke), p(s)
            ),
            ShapeType::RoundedRectangle(stroke, s, corner_radius) => {
                let corner_radius = size.scale_physical(corner_radius);
                wgpu_canvas::Shape::RoundedRectangle(
                    size.scale_physical(stroke), p(s), corner_radius
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    pub text: &'static str,
    pub color: &'static str,
    pub alpha: u8,
    pub width: Option<u32>,
    pub size: u32,
    pub line_height: u32,
    pub font: Font,

}

impl Text {
    pub fn new(
        text: &'static str,
        color: &'static str,
        alpha: u8,
        width: Option<u32>,
        size: u32,
        line_height: u32,
        font: Font,
    ) -> Self {
        Text{text, color, alpha, width, size, line_height, font}
    }

    pub fn into_inner(self, size: &Size) -> wgpu_canvas::Text {
        let ce = "Color was not a Hex Value";
        let c: [u8; 3] = hex::decode(self.color).expect(ce).try_into().expect(ce);
        wgpu_canvas::Text::new(
            self.text,
            (c[0], c[1], c[2], self.alpha),
            self.width.map(|w| size.scale_physical(w)),
            size.scale_physical(self.size),
            size.scale_physical(self.line_height),
            self.font
        )
    }
}

#[derive(Debug, Clone)]
pub enum CanvasItem {
    Shape(ShapeType, &'static str, u8),
    Image(ShapeType, Image),
    Text(Text)
}

impl CanvasItem {
    pub fn into_inner(self, size: &Size) -> wgpu_canvas::CanvasItem {
        match self {
            CanvasItem::Shape(shape, color, alpha) => {
                let ce = "Color was not a Hex Value";
                let c: [u8; 3] = hex::decode(color).expect(ce).try_into().expect(ce);
                wgpu_canvas::CanvasItem::Shape(
                    shape.into_inner(size), (c[0], c[1], c[2], alpha)
                )
            },
            CanvasItem::Image(shape, image) => {
                wgpu_canvas::CanvasItem::Image(shape.into_inner(size), image)
            },
            CanvasItem::Text(text) => {
                wgpu_canvas::CanvasItem::Text(text.into_inner(size))
            }
        }

    }
}

//  pub struct Image(wgpu_canvas::Image);

//  impl Image {
//      pub fn new(ctx: &mut CanvasContext, image: image::RgbaImage) -> Self {
//          Image(wgpu_canvas::Image::new(ctx.atlas, image))
//      }
//      pub fn new_sized(ctx: &mut CanvasContext, image: image::RgbaImage, size: (u32, u32)) -> Self {
//          let mut dst_image = DynamicImage::new_rgba8(size.0, size.1);
//          Resizer::new().resize(
//              &DynamicImage::from(image),
//              &mut dst_image,
//              &ResizeOptions::new()
//                  .resize_alg(ResizeAlg::SuperSampling(FilterType::Bilinear, 8))
//                  .fit_into_destination(Some((0.5, 0.5))),
//          ).unwrap();
//          Image::new(ctx, dst_image.into())
//      }

//      fn from_svg(&mut self, bytes: &[u8], min_size: u32, color: &'static str) -> RgbaImage {
//          let size = self.size.scale_physical(size);
//          let mut content = std::str::from_utf8(bytes).unwrap();
//          content = content.replace("fill=\"white\"", &format!("fill=\"#{}\"", color));
//          let svg = nsvg::parse_str(&content, nsvg::Units::Pixel, 96.0).unwrap();
//          let rgba = svg.rasterize(min_size as f32/ svg.width().min(svg.height).ceil()).unwrap();
//          Self::new_sized(RgbaImage::from_raw(rgba.dimensions().0, rgba.dimensions().1, rgba.into_raw()).unwrap())
//      }

//      pub fn into_inner(self) -> wgpu_canvas::Image {
//          self.0
//      }
//  }

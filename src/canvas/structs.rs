use super::{ImageKey, FontKey};

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
    pub fn to_logical(&self, px: u32, py: u32) -> (u32, u32) {
        (
            (px.min(self.width) as f64 / self.scale_factor).floor() as u32,
            (py.min(self.height) as f64 / self.scale_factor).floor() as u32,
        )
    }

    pub fn scale_physical(&self, x: u32) -> u32 {
        (x as f64 * self.scale_factor).round() as u32
    }

    pub fn to_physical(&self, lx: u32, ly: u32) -> (u32, u32) {
        (
            ((lx as f64 * self.scale_factor).round() as u32).min(self.width),
            ((ly as f64 * self.scale_factor).round() as u32).min(self.height),
        )
    }

    pub fn logical(&self) -> (u32, u32) {
        self.to_logical(self.width, self.height)
    }

    pub fn physical(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Area(pub (u32, u32), pub Option<(u32, u32, u32, u32)>);

impl Area {
    pub(crate) fn into_inner(self, z_index: u16, size: Size) -> wgpu_canvas::Area {
        let psize = size.physical();
        let bounds = self.1.map(|(x, y, w, h)| {
            let xy = size.to_physical(x, y);
            let wh = size.to_physical(w, h);
            (xy.0, xy.1, wh.0, wh.1)
        }).unwrap_or((0, 0, psize.0, psize.1));
        wgpu_canvas::Area{
            z_index,
            bounds,
            offset: size.to_physical(self.0.0, self.0.1),
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
    pub(crate) fn into_inner(self, size: Size) -> wgpu_canvas::shape::ShapeType {
        let p = |s: (u32, u32)| size.to_physical(s.0, s.1);
        let st = |s: u32| size.to_physical(s, s);
        match self {
            Shape::Ellipse(stroke, s) => wgpu_canvas::shape::ShapeType::Ellipse(
                wgpu_canvas::shape::Ellipse{stroke: st(stroke), size: p(s)}
            ),
            Shape::Rectangle(stroke, s) => wgpu_canvas::shape::ShapeType::Rectangle(
                wgpu_canvas::shape::Rectangle{stroke: st(stroke), size: p(s)}
            ),
            Shape::RoundedRectangle(stroke, s, corner_radius) => {
                let corner_radius = size.scale_physical(corner_radius);
              //let pcr = size.to_physical(corner_radius, corner_radius);
              //let size = p(s);
              //let corner_radius = if size.0 > size.1 {pcr.0} else {pcr.1};

                wgpu_canvas::shape::ShapeType::RoundedRectangle(
                    wgpu_canvas::shape::RoundedRectangle{
                        shape: wgpu_canvas::shape::GenericShape{
                            stroke: st(stroke),
                            size: p(s)
                        },
                        corner_radius
                    }
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Text {
    pub text: &'static str,
    pub color: &'static str,
    pub alpha: u8,
    pub width: Option<u32>,
    pub size: u32,
    pub line_height: u32,
    pub font: FontKey,
}

impl Text {
    pub fn new(
        text: &'static str,
        color: &'static str,
        alpha: u8,
        width: Option<u32>,
        size: u32,
        line_height: u32,
        font: FontKey,
    ) -> Self {
        Text{text, color, alpha, width, size, line_height, font}
    }

    pub(crate) fn into_inner(self, size: Size) -> wgpu_canvas::text::Text {
        let ce = "Color was not a Hex Value";
        let c: [u8; 3] = hex::decode(self.color).expect(ce).try_into().expect(ce);
        wgpu_canvas::text::Text{
            text: self.text,
            color: (c[0], c[1], c[2], self.alpha),
            width: self.width.map(|w| size.scale_physical(w)),
            size: size.scale_physical(self.size),
            line_height: size.scale_physical(self.line_height),
            font: self.font
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CanvasItem {
    Shape(Area, Shape, &'static str, u8),
    Image(Area, Shape, ImageKey),
    Text(Area, Text),
}

impl CanvasItem {
    pub fn area(&mut self) -> &mut Area {
        match self {
            CanvasItem::Shape(area, ..) => area,
            CanvasItem::Image(area, ..) => area,
            CanvasItem::Text(area, ..) => area,
        }
    }

    pub(crate) fn into_inner(self, z_index: u16, size: Size) -> wgpu_canvas::CanvasItem {
        let (area, item_type) = match self {
            CanvasItem::Shape(area, shape, color, alpha) => {
                let ce = "Color was not a Hex Value";
                let c: [u8; 3] = hex::decode(color).expect(ce).try_into().expect(ce);
                (
                    area,
                    wgpu_canvas::ItemType::ColorShape(
                        wgpu_canvas::color::ColorShape(
                            wgpu_canvas::color::Color(c[0], c[1], c[2], alpha),
                            shape.into_inner(size)
                        )
                    )
                )
            },
            CanvasItem::Image(area, shape, image) => {
                (
                    area,
                    wgpu_canvas::ItemType::ImageShape(
                        wgpu_canvas::image::ImageShape(
                            image, shape.into_inner(size)
                        )
                    )
                )
            },
            CanvasItem::Text(area, text) => {
                (
                    area,
                    wgpu_canvas::ItemType::Text(
                        text.into_inner(size)
                    )
                )
            }
        };

        wgpu_canvas::CanvasItem{
            area: area.into_inner(z_index, size),
            item_type
        }
    }
}

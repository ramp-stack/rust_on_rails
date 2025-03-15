use super::{CanvasContext, Image, Font};

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
    pub(crate) fn into_inner(self, size: Size) -> wgpu_canvas::Shape {
        let p = |s: (u32, u32)| size.to_physical(s.0, s.1);
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
}

//CanvasItems:
//1. Cheap to clone
//2. Can only modify cheap to change fields
//3. Requires context for new

#[derive(Debug, Clone)]
pub struct CanvasItem(pub(crate) wgpu_canvas::CanvasItem);

impl CanvasItem {
    pub fn shape(ctx: &mut CanvasContext, shape: Shape, color: &'static str, alpha: u8) -> Self {
        let ce = "Color was not a Hex Value";
        let c: [u8; 3] = hex::decode(color).expect(ce).try_into().expect(ce);
        CanvasItem(wgpu_canvas::CanvasItem::shape(
            &mut ctx.atlas, shape.into_inner(ctx.size), (c[0], c[1], c[2], alpha)
        ))
    }

    pub fn image(ctx: &mut CanvasContext, shape: Shape, image: Image) -> Self {
        CanvasItem(wgpu_canvas::CanvasItem::image(
            &mut ctx.atlas, shape.into_inner(ctx.size), image
        ))
    }

    pub fn text(
        ctx: &mut CanvasContext,
        text: &'static str,
        color: &'static str,
        alpha: u8,
        width: Option<u32>,
        s: u32,
        line_height: u32,
        font: Font,
    ) -> Self {
        let ce = "Color was not a Hex Value";
        let c: [u8; 3] = hex::decode(color).expect(ce).try_into().expect(ce);
        CanvasItem(wgpu_canvas::CanvasItem::text(
            text,
            (c[0], c[1], c[2], alpha),
            width.map(|w| ctx.size.scale_physical(w)),
            ctx.size.scale_physical(s),
            ctx.size.scale_physical(line_height),
            font
        ))
    }

    pub fn size(&self, ctx: &mut CanvasContext) -> (u32, u32) {
        self.0.size(&mut ctx.atlas)
    }
}

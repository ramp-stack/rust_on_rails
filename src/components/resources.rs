use super::ComponentContext;
use crate::canvas;


#[derive(Debug, Clone)]
pub struct Image(canvas::Image);
impl Image {
    pub fn new(ctx: &mut ComponentContext, image: image::RgbaImage) -> Self {
        Image(canvas::Image::new(ctx.canvas, image))
    }

    pub fn svg(ctx: &mut ComponentContext, svg: &[u8], scale: f32) -> Self {
        Image(canvas::Image::svg(ctx.canvas, svg, scale))
    }

    pub(crate) fn into_inner(self) -> canvas::Image {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Font(canvas::Font);
impl Font {
    pub fn new(ctx: &mut ComponentContext, font: Vec<u8>) -> Self {
        Font(canvas::Font::new(ctx.canvas, font))
    }

    pub(crate) fn into_inner(self) -> canvas::Font {
        self.0
    }
}

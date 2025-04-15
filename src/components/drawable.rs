use crate::canvas;
use crate::canvas::{CanvasItem, Color};

use std::fmt::Debug;

use super::resources;
use super::events::*;
use super::sizing::*;
use super::{ComponentContext};


#[derive(Default, Debug, Clone)]
pub struct RequestBranch(pub SizeRequest, Vec<RequestBranch>);

#[derive(Default, Debug, Clone)]
pub struct SizedBranch(pub Size, Vec<(Offset, SizedBranch)>);

type Offset = (f32, f32);
type Rect = (f32, f32, f32, f32);
type Size = (f32, f32);

#[allow(private_bounds)]
pub trait Drawable: _Drawable + Debug {
    fn request_size(&self, ctx: &mut ComponentContext) -> SizeRequest;
    fn name(&self) -> String;
}
impl<D: _Drawable + ?Sized> Drawable for D {
    fn request_size(&self, ctx: &mut ComponentContext) -> SizeRequest {_Drawable::request_size(self, ctx).0}
    fn name(&self) -> String {_Drawable::name(self)}
}

pub(crate) trait _Drawable: Debug {
    fn request_size(&self, ctx: &mut ComponentContext) -> RequestBranch;
    fn build(&mut self, _ctx: &mut ComponentContext, size: Size, request: RequestBranch) -> SizedBranch {
        SizedBranch(request.0.get(size), vec![])
    }
    fn draw(&mut self, ctx: &mut ComponentContext, sized: SizedBranch, offset: Offset, bound: Rect);

    fn name(&self) -> String {std::any::type_name_of_val(self).to_string()}

    fn event(&mut self, _ctx: &mut ComponentContext, _sized: SizedBranch, _event: Box<dyn Event>) {}
}

#[derive(Clone, Debug)]
pub struct Text{
    pub text: String,
    pub color: Color,
    pub max_width: Option<f32>,
    pub font_size: f32,
    pub line_height: f32,
    pub font: resources::Font
}

impl Text {
    pub fn new(text: &str, color: Color, max_width: Option<f32>, font_size: f32, line_height: f32, font: resources::Font) -> Self {
        Text{text: text.to_string(), color, max_width, font_size, line_height, font}
    }

    fn into_inner(self) -> canvas::Text {
        canvas::Text{text: self.text, color: self.color, width: self.max_width, size: self.font_size, line_height: self.line_height, font: self.font.clone().into_inner()}
    }
}

impl _Drawable for Text {
    fn request_size(&self, ctx: &mut ComponentContext) -> RequestBranch {
        RequestBranch(SizeRequest::fixed(self.clone().into_inner().size(ctx.canvas)), vec![])
    }

    fn draw(&mut self, ctx: &mut ComponentContext, _sized: SizedBranch, offset: Offset, bound: Rect) {
        ctx.canvas.draw(canvas::Area(offset, Some(bound)), CanvasItem::Text(self.clone().into_inner()))
    }

    fn event(&mut self, _ctx: &mut ComponentContext, _sized: SizedBranch, _event: Box<dyn Event>) {
        // if let Ok(event) = event.downcast::<MouseEvent>() {
        //     if event.state == MouseState::Pressed && event.position.is_some() {
        //         if self.color.0 > 0 && self.color.1 == 0 {self.color = Color(0, 255, 0, 255)}
        //         else if self.color.0 > 0 && self.color.1 > 0 {self.color = Color(255, 0, 0, 255)}
        //         else if self.color.1 > 0 {self.color = Color(0, 0, 255, 255)}
        //         else if self.color.2 > 0 {self.color = Color(255, 255, 255, 255)}
        //     }
        // }
    }
}

pub use canvas::Shape as ShapeType;

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub shape: ShapeType,
    pub color: Color
}
impl _Drawable for Shape {
    fn request_size(&self, _ctx: &mut ComponentContext) -> RequestBranch {RequestBranch(SizeRequest::fixed(self.shape.size()), vec![])}

    fn draw(&mut self, ctx: &mut ComponentContext, _sized: SizedBranch, offset: Offset, bound: Rect) {//TODO: use sized.0 as the size of the shape?
        ctx.canvas.draw(canvas::Area(offset, Some(bound)), CanvasItem::Shape(self.shape, self.color))
    }
}

#[derive(Clone, Debug)]
pub struct Image {
    pub shape: ShapeType,
    pub image: resources::Image,
    pub color: Option<Color>
}

impl _Drawable for Image {
    fn request_size(&self, _ctx: &mut ComponentContext) -> RequestBranch {RequestBranch(SizeRequest::fixed(self.shape.size()), vec![])}

    fn draw(&mut self, ctx: &mut ComponentContext, _sized: SizedBranch, offset: Offset, bound: Rect) {
        ctx.canvas.draw(canvas::Area(offset, Some(bound)), CanvasItem::Image(self.shape, self.image.clone().into_inner(), self.color))
    }
}

pub trait Component: Debug {
    fn children_mut(&mut self) -> Vec<&mut dyn Drawable>;
    fn children(&self) -> Vec<&dyn Drawable>;

    fn request_size(&self, ctx: &mut ComponentContext, children: Vec<SizeRequest>) -> SizeRequest;
    fn build(&mut self, ctx: &mut ComponentContext, size: Size, children: Vec<SizeRequest>) -> Vec<Area>;
}

impl<C: Component + ?Sized + 'static + Events> _Drawable for C {
    fn request_size(&self, ctx: &mut ComponentContext) -> RequestBranch {
        let requests = self.children().into_iter().map(|i| _Drawable::request_size(i, ctx)).collect::<Vec<_>>();
        let info = requests.iter().map(|i| i.0).collect::<Vec<_>>();
        RequestBranch(Component::request_size(self, ctx, info), requests)
    }

    fn build(&mut self, ctx: &mut ComponentContext, size: Size, request: RequestBranch) -> SizedBranch {
        let size = request.0.get(size);
        let children = request.1.iter().map(|b| b.0).collect::<Vec<_>>();
        SizedBranch(
            size,
            Component::build(self, ctx, size, children).into_iter()
            .zip(self.children_mut()).zip(request.1)
            .map(|((Area{offset, size}, child), branch)| {
                (offset, child.build(ctx, size, branch))
            }).collect()
        )
    }

    fn draw(&mut self, ctx: &mut ComponentContext, sized: SizedBranch, poffset: Offset, bound: Rect) {
        sized.1.into_iter().zip(self.children_mut()).for_each(|((offset, branch), child)| {
            let size = branch.0;
            let poffset = (poffset.0+offset.0, poffset.1+offset.1);

            let bound = (
                bound.0.max(poffset.0), bound.1.max(poffset.1),//New bound offset
                bound.2.min((offset.0 + size.0).max(0.0)), bound.3.min((offset.1 + size.1).max(0.0))//New bound size
            );

            if bound.2 != 0.0 && bound.3 != 0.0 {
                child.draw(ctx, branch, poffset, bound);
            }
        })
    }

    fn event(&mut self, ctx: &mut ComponentContext, sized: SizedBranch, mut event: Box<dyn Event>) {
        if Events::on_event(self, ctx, &mut *event) {
            let children = sized.1.iter().map(|(o, branch)| (*o, branch.0)).collect::<Vec<_>>();
            event.pass(ctx, children).into_iter().zip(self.children_mut()).zip(sized.1).for_each(
                |((e, child), branch)| if let Some(e) = e {child.event(ctx, branch.1, e);}
            );
        }
    }
}

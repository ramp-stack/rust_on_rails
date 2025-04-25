use crate::base;
use base::renderer::wgpu_canvas as canvas;
use canvas::CanvasItem;
use canvas::Area as CanvasArea;

use std::fmt::Debug;
use std::any::Any;

use super::{Context, resources};
use super::events::*;
use super::sizing::*;

pub use canvas::{Text, Font, Span, Align, Cursor, Color};

#[derive(Default, Debug, Clone)]
pub struct RequestBranch(pub SizeRequest, Vec<RequestBranch>);

#[derive(Default, Debug, Clone)]
pub struct SizedBranch(pub Size, Vec<(Offset, SizedBranch)>);

type Offset = (f32, f32);
type Rect = (f32, f32, f32, f32);
type Size = (f32, f32);

#[allow(private_bounds)]
pub trait Drawable: _Drawable + Debug + Any {
    fn request_size(&self, ctx: &mut Context) -> SizeRequest;
    fn name(&self) -> String;

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<D: _Drawable + Debug + Any> Drawable for D {
    fn request_size(&self, ctx: &mut Context) -> SizeRequest {_Drawable::request_size(self, ctx).0}
    fn name(&self) -> String {_Drawable::name(self)}
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

pub(crate) trait _Drawable: Debug {
    fn request_size(&self, ctx: &mut Context) -> RequestBranch;
    fn build(&mut self, _ctx: &mut Context, size: Size, request: RequestBranch) -> SizedBranch {
        SizedBranch(request.0.get(size), vec![])
    }
    fn draw(&mut self, ctx: &mut Context, sized: SizedBranch, offset: Offset, bound: Rect);

    fn name(&self) -> String {std::any::type_name_of_val(self).to_string()}

    fn event(&mut self, _ctx: &mut Context, _sized: SizedBranch, _event: Box<dyn Event>) {}
}

impl _Drawable for Text {
    fn request_size(&self, ctx: &mut Context) -> RequestBranch {
        let size = self.size(ctx.as_canvas());
        RequestBranch(SizeRequest::fixed(size), vec![])
    }

    fn draw(&mut self, ctx: &mut Context, _sized: SizedBranch, offset: Offset, bound: Rect) {
        ctx.as_canvas().draw(CanvasArea(offset, Some(bound)), CanvasItem::Text(self.clone()));
    }

    fn event(&mut self, _ctx: &mut Context, _sized: SizedBranch, event: Box<dyn Event>) {
       if let Ok(event) = event.downcast::<MouseEvent>() {
           if event.state == MouseState::Pressed && event.position.is_some() {
               self.spans.iter_mut().for_each(|s| {
                   let color = &mut s.color;
                   if color.0 > 0 && color.1 == 0 {*color = Color(0, 255, 0, 255)}
                   else if color.0 > 0 && color.1 > 0 {*color = Color(255, 0, 0, 255)}
                   else if color.1 > 0 {*color = Color(0, 0, 255, 255)}
                   else if color.2 > 0 {*color = Color(255, 255, 255, 255)}
                });
           }
       }
    }
}

pub use canvas::Shape as ShapeType;

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub shape: ShapeType,
    pub color: Color
}
impl _Drawable for Shape {
    fn request_size(&self, _ctx: &mut Context) -> RequestBranch {RequestBranch(SizeRequest::fixed(self.shape.size()), vec![])}

    fn draw(&mut self, ctx: &mut Context, _sized: SizedBranch, offset: Offset, bound: Rect) {
        //TODO: use sized.0 as the size of the shape?
       ctx.as_canvas().draw(CanvasArea(offset, Some(bound)), CanvasItem::Shape(self.shape, self.color));
    }
}

#[derive(Clone, Debug)]
pub struct Image {
    pub shape: ShapeType,
    pub image: resources::Image,
    pub color: Option<Color>
}

impl _Drawable for Image {
    fn request_size(&self, _ctx: &mut Context) -> RequestBranch {RequestBranch(SizeRequest::fixed(self.shape.size()), vec![])}

    fn draw(&mut self, ctx: &mut Context, _sized: SizedBranch, offset: Offset, bound: Rect) {
        ctx.as_canvas().draw(CanvasArea(offset, Some(bound)), CanvasItem::Image(self.shape, self.image.clone(), self.color));
    }
}

pub trait Component: Debug {
    fn children_mut(&mut self) -> Vec<&mut dyn Drawable>;
    fn children(&self) -> Vec<&dyn Drawable>;

    fn request_size(&self, ctx: &mut Context, children: Vec<SizeRequest>) -> SizeRequest;
    fn build(&mut self, ctx: &mut Context, size: Size, children: Vec<SizeRequest>) -> Vec<Area>;
}

impl<C: Component + ?Sized + 'static + OnEvent> _Drawable for C {
    fn request_size(&self, ctx: &mut Context) -> RequestBranch {
        let requests = self.children().into_iter().map(|i| _Drawable::request_size(i, ctx)).collect::<Vec<_>>();
        let info = requests.iter().map(|i| i.0).collect::<Vec<_>>();
        RequestBranch(Component::request_size(self, ctx, info), requests)
    }

    fn build(&mut self, ctx: &mut Context, size: Size, request: RequestBranch) -> SizedBranch {
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

    fn draw(&mut self, ctx: &mut Context, sized: SizedBranch, poffset: Offset, bound: Rect) {
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

    fn event(&mut self, ctx: &mut Context, sized: SizedBranch, mut event: Box<dyn Event>) {
        if OnEvent::on_event(self, ctx, &mut *event) {
            let children = sized.1.iter().map(|(o, branch)| (*o, branch.0)).collect::<Vec<_>>();
            event.pass(ctx, children).into_iter().zip(self.children_mut()).zip(sized.1).for_each(
                |((e, child), branch)| if let Some(e) = e {child.event(ctx, branch.1, e);}
            );
        }
    }
}

use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, Area};
use crate::canvas;

use include_dir::{DirEntry, Dir};

use std::collections::HashMap;

pub mod resources;
use resources::Font;

pub use canvas::Color;

pub type BoxComponent = Box<dyn Drawable>;
pub type ComponentRef<'a> = dyn Drawable + 'a;

pub type Rect = (i32, i32, u32, u32);

pub trait Plugin {
    fn name() -> &'static str where Self: Sized;
}

pub struct ComponentContext<'a> {
    plugins: &'a mut HashMap<&'static str, Box<dyn std::any::Any>>,
    assets: &'a mut Vec<Dir<'static>>,
    canvas: &'a mut CanvasContext
}

impl<'a> ComponentContext<'a> {
    pub fn new(
        plugins: &'a mut HashMap<&'static str, Box<dyn std::any::Any>>,
        assets: &'a mut Vec<Dir<'static>>,
        canvas: &'a mut CanvasContext
    ) -> Self {
        ComponentContext{plugins, assets, canvas}
    }

    pub fn configure_plugin<P: Plugin + 'static>(&mut self, plugin: P) {
        self.plugins.insert(P::name(), Box::new(plugin));
    }

    pub fn get<P: Plugin + 'static>(&mut self) -> &mut P {
        self.plugins.get_mut(P::name())
            .unwrap_or_else(|| panic!("Plugin Not Configured: {}", P::name()))
            .downcast_mut().unwrap()
    }

    pub fn include_assets(&mut self, dir: Dir<'static>) {
        self.assets.push(dir);
    }

    pub fn load_file(&self, file: &str) -> Option<Vec<u8>> {
        self.assets.iter().find_map(|dir|
            dir.find(file).ok().and_then(|mut f|
                f.next().and_then(|f|
                    if let DirEntry::File(f) = f {Some(f.contents().to_vec())} else {None}
                )
            )
        )
    }
}

pub trait ComponentAppTrait {
    fn new(ctx: &mut ComponentContext) -> impl std::future::Future<Output = Box<dyn Drawable>> where Self: Sized;
}

pub struct ComponentApp<A: ComponentAppTrait> {
    plugins: HashMap<&'static str, Box<dyn std::any::Any>>,
    assets: Vec<Dir<'static>>,
    app: Box<dyn Drawable>,
    _p: std::marker::PhantomData<A>
}

impl<A: ComponentAppTrait> CanvasAppTrait for ComponentApp<A> {
    async fn new(ctx: &mut CanvasContext) -> Self {
        let mut plugins = HashMap::new();
        let mut assets = Vec::new();
        let mut ctx = ComponentContext::new(&mut plugins, &mut assets, ctx);
        let app = A::new(&mut ctx).await;
        ComponentApp{plugins, assets, app, _p: std::marker::PhantomData::<A>}
    }

    async fn on_tick(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.width();
        let height = ctx.height();
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_tick(&mut ctx);
        self.app.draw(&mut ctx, (0, 0, width, height), (0, 0, width, height));
    }

    async fn on_click(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.width();
        let height = ctx.height();
        let (x, y) = ctx.mouse();
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_click(&mut ctx, (width, height), Some((x, y)));
    }

    async fn on_move(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.width();
        let height = ctx.height();
        let (x, y) = ctx.mouse();
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_move(&mut ctx, (width, height), Some((x, y)));
    }

    async fn on_press(&mut self, _ctx: &mut CanvasContext, _t: String) {}
}

#[macro_export]
macro_rules! create_component_entry_points {
    ($app:ty) => {
        create_canvas_entry_points!(ComponentApp::<$app>);
    };
}

pub trait Drawable {
    fn size(&self, ctx: &mut ComponentContext) -> (Option<u32>, Option<u32>);
    fn draw(&self, ctx: &mut ComponentContext, position: Rect, bound: Rect);

    fn on_tick(&mut self, _ctx: &mut ComponentContext) {}
    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: Option<(u32, u32)>) {}
    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: Option<(u32, u32)>) {}

}

// Text, Color, Opacity, text size, line height, font
#[derive(Clone, Debug)]
pub struct Text(pub String, pub Color, pub Option<u32>, pub u32, pub u32, pub Font);
impl Text {
    pub fn new(text: &str, color: Color, width: Option<u32>, size: u32, line_height: u32, font: Font) -> Self {
        Text(text.to_string(), color, width, size, line_height, font)
    }

    fn into_inner(self) -> canvas::Text {
        canvas::Text{text: self.0, color: self.1, width: self.2, size: self.3, line_height: self.4, font: self.5.clone().into_inner()}
    }
}

impl Drawable for Text {
    fn size(&self, ctx: &mut ComponentContext) -> (Option<u32>, Option<u32>) {
        let size = self.clone().into_inner().size(ctx.canvas);
        (Some(size.0), Some(size.1))
    }

    fn draw(&self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        ctx.canvas.draw(Area((position.0, position.1), Some(bound)), CanvasItem::Text(self.clone().into_inner()))
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), position: Option<(u32, u32)>) {
        if position.is_some() {
            if self.1.0 > 0 {self.1 = Color(0, 255, 0, 255)}
            else if self.1.1 > 0 {self.1 = Color(0, 0, 255, 255)}
            else if self.1.2 > 0 {self.1 = Color(255, 0, 0, 255)}
        }
        println!("Text: {:?}", position);
    }
}

pub use canvas::Shape as ShapeType;

#[derive(Clone, Copy, Debug)]
pub struct Shape(pub ShapeType, pub Color);

impl Drawable for Shape {
    fn size(&self, _ctx: &mut ComponentContext) -> (Option<u32>, Option<u32>) {
        let size = self.0.size();
        (Some(size.0), Some(size.1))
    }

    fn draw(&self, ctx: &mut ComponentContext, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Shape(self.0, self.1))
    }
}

#[derive(Clone, Debug)]
pub struct Image(pub ShapeType, pub resources::Image, pub Option<Color>);

impl Drawable for Image {
    fn size(&self, _ctx: &mut ComponentContext) -> (Option<u32>, Option<u32>) {
        let size = self.0.size();
        (Some(size.0), Some(size.1))
    }

    fn draw(&self, ctx: &mut ComponentContext, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Image(self.0, self.1.clone().into_inner(), self.2))
    }
}

pub type SizeFn<'a> = Box<dyn FnMut(&mut ComponentContext, (u32, u32)) -> (u32, u32) + 'a>;

pub trait Component {
    fn children_mut(&mut self) -> Vec<&mut dyn Drawable>;
    fn children(&self) -> Vec<&dyn Drawable>;
    fn layout(&self) -> &dyn Layout;

    fn size(&self, ctx: &mut ComponentContext) -> (Option<u32>, Option<u32>) {
        let sizes = self.children().into_iter().map(|i| i.size(ctx)).collect();
        self.layout().size(ctx, sizes)
    }

    fn on_tick(&mut self, _ctx: &mut ComponentContext) {}
    fn on_click(&mut self, _ctx: &mut ComponentContext, _position: Option<(u32, u32)>) -> bool {true}
    fn on_move(&mut self, _ctx: &mut ComponentContext, _position: Option<(u32, u32)>) -> bool {true}
}

trait _Component: Component {
    fn build(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> Vec<((i32, i32), (u32, u32))> {
        let sizes = self.children().into_iter().map(|i| i.size(ctx)).collect();
        self.layout().build(ctx, max_size, sizes)
    }

    fn pass_event(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: Option<(u32, u32)>, on_click: bool) {
        let mut clicked = false;
        self.build(ctx, max_size).into_iter().zip(self.children_mut()).rev().for_each(|((offset, size), child)| {//Reverse to click on the top most element
            let position = position.and_then(|position| {
                if !clicked && (position.0 as i32) > offset.0 && (position.0 as i32) < offset.0+size.0 as i32 && (position.1 as i32) > offset.1 && (position.1 as i32) < offset.1+size.1 as i32 {
                    clicked = true;
                    Some((position.0-offset.0 as u32, position.1-offset.1 as u32))
                } else {None}
            });
            if on_click { child.on_click(ctx, size, position); } else { child.on_move(ctx, size, position); }
        })
    }
}
impl<C: Component + ?Sized> _Component for C {}

impl<C: _Component + ?Sized + 'static> Drawable for C {
    fn draw(&self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        let max_size = (position.2, position.3);
        self.build(ctx, max_size).into_iter().zip(self.children()).for_each(|((offset, size), child)| {
            let position = (
                position.0+offset.0, position.1+offset.1,//Screen Offset Total
                size.0, size.1//Size of underlaying component
            );

            let bound = (
                bound.0.max(bound.0+offset.0), bound.1.max(bound.1+offset.1),//New bound offset
                bound.2.min((offset.0 + size.0 as i32).max(0) as u32), bound.3.min((offset.1 + size.1 as i32).max(0) as u32)//New bound size
            );

            if bound.2 != 0 && bound.3 != 0 {
                child.draw(ctx, position, bound);
            }
        })
    }

    fn size(&self, ctx: &mut ComponentContext) -> (Option<u32>, Option<u32>) {Component::size(self, ctx)}
    fn on_tick(&mut self, ctx: &mut ComponentContext) {
        Component::on_tick(self, ctx);
        self.children_mut().into_iter().for_each(|c| c.on_tick(ctx));
    }
    fn on_click(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: Option<(u32, u32)>) {
        let position = Component::on_click(self, ctx, position).then(|| position).flatten();
        self.pass_event(ctx, max_size, position, true)
    }
    fn on_move(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: Option<(u32, u32)>) {
        let position = Component::on_move(self, ctx, position).then(|| position).flatten();
        self.pass_event(ctx, max_size, position, false)
    }
}

pub trait Layout {
    fn build(&self, ctx: &mut ComponentContext, max_size: (u32, u32), items: Vec<(Option<u32>, Option<u32>)>) -> Vec<((i32, i32), (u32, u32))>;
    fn size(&self, ctx: &mut ComponentContext, items: Vec<(Option<u32>, Option<u32>)>) -> (Option<u32>, Option<u32>);
}

#[derive(Clone, Debug)]
pub struct DefaultLayout;
impl Layout for DefaultLayout {
    fn build(&self, _ctx: &mut ComponentContext, max_size: (u32, u32), items: Vec<(Option<u32>, Option<u32>)>) -> Vec<((i32, i32), (u32, u32))> {
        items.into_iter().map(|(w, h)| ((0, 0), (w.unwrap_or(max_size.0), h.unwrap_or(max_size.1)))).collect()
    }

    fn size(&self, _ctx: &mut ComponentContext, items: Vec<(Option<u32>, Option<u32>)>) -> (Option<u32>, Option<u32>) {
        items.into_iter().fold((Some(0), Some(0)), |(ow, oh), (w, h)| {
            (
                ow.and_then(|ow| w.map(|w| ow.max(w))),
                oh.and_then(|oh| h.map(|h| oh.max(h))),
            )
        })
    }
}


//  //  pub struct Container<'a>(Box<dyn Layout>, Vec<&'a mut dyn Drawable>);
//  //  impl<'a> Container<'a> {
//  //      pub fn new(layout: impl Layout + 'static, items: Vec<&'a mut dyn Drawable>) -> Self {
//  //          Container(Box::new(layout), items)
//  //      }
//  //  }
//  //  #[macro_export]
//  //  macro_rules! container {
//  //      [$($child:expr),* $(,)?] => {{
//  //          Container::new(DefaultLayout, vec![ $($child as &mut dyn Drawable),* ])
//  //      }};
//  //  }

//  pub struct BuiltComponent<'a>(Vec<((i32, i32), (u32, u32), &'a mut dyn Drawable)>);

//  impl<'a> BuiltComponent<'a> {
//      fn new(ctx: &mut ComponentContext, max_size: (u32, u32)) -> Self {
//          let size_fns = container.1.iter_mut().map(|c| Box::new(|ctx: &mut ComponentContext, max: (u32, u32)| c.size(ctx, max)) as SizeFn).collect();
//          BuiltComponent(container.0.build(ctx, max_size, size_fns).into_iter().zip(container.1).map(|((offset, size), child)| (offset, size, child)).collect())
//      }

//      fn pass_event(&mut self, ctx: &mut ComponentContext, position: Option<(u32, u32)>, on_click: bool) {
//          let mut clicked = false;
//          self.0.iter_mut().rev().for_each(|(offset, size, child)| {//Reverse to click on the top most element
//              let position = position.and_then(|position| {
//                  if !clicked && (position.0 as i32) > offset.0 && (position.0 as i32) < offset.0+size.0 as i32 && (position.1 as i32) > offset.1 && (position.1 as i32) < offset.1+size.1 as i32 {
//                      clicked = true;
//                      Some((position.0-offset.0 as u32, position.1-offset.1 as u32))
//                  } else {None}
//              });
//              if on_click { child.on_click(ctx, *size, position); } else { child.on_move(ctx, *size, position); }
//          })
//      }

//      fn size(&self, ctx: &mut ComponentContext) -> (u32, u32) {
//          self.0.iter().fold((0, 0), |old_size, (offset, size, _)| {
//              let size = ((offset.0 + size.0 as i32).max(0) as u32, (offset.1 + size.1 as i32).max(0) as u32);
//              (old_size.0.max(size.0), old_size.1.max(size.1))
//          })
//      }

//      fn draw(&mut self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
//          self.0.iter_mut().for_each(|(offset, size, child)| {
//              let position = (
//                  position.0+offset.0, position.1+offset.1,//Screen Offset Total
//                  size.0, size.1//Size of underlaying component
//              );

//              let bound = (
//                  bound.0.max(bound.0+offset.0), bound.1.max(bound.1+offset.1),//New bound offset
//                  bound.2.min((offset.0 + size.0 as i32).max(0) as u32), bound.3.min((offset.1 + size.1 as i32).max(0) as u32)//New bound size
//              );

//              if bound.2 != 0 && bound.3 != 0 {
//                  child.draw(ctx, position, bound);
//              }
//          })
//      }

//      fn on_click(&mut self, ctx: &mut ComponentContext, position: Option<(u32, u32)>) {
//          self.pass_event(ctx, position, true);
//      }

//      fn on_move(&mut self, ctx: &mut ComponentContext, position: Option<(u32, u32)>) {
//          self.pass_event(ctx, position, false);
//      }
//  }

//  #[derive(Clone, Copy, Default, Debug)]
//  pub enum Size {
//      #[default]
//      Fit,
//      Expand,
//      Fill,
//      Static(u32, u32),
//      Custom(fn((u32, u32), (u32, u32)) -> (u32, u32))
//  }

//  impl Size {
//      pub fn get(&self, max_size: (u32, u32)) -> (u32, u32) {
//          match self {
//              Self::Fit => ,
//              Self::Expand => c_size,
//              Self::Fill => max_size,
//              Self::Static(x, y) => (*x, *y),
//              Self::Custom(func) => func(max_size, c_size)
//          }
//      }
//  }

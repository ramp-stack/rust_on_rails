use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, Area};
use crate::canvas;

use include_dir::{DirEntry, Dir};

use std::collections::HashMap;

pub mod resources;
use resources::Font;

pub use canvas::Color;

pub type BoxComponent = Box<dyn Drawable>;

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
    fn new(ctx: &mut ComponentContext) -> impl std::future::Future<Output = BoxComponent> where Self: Sized;
}

pub struct ComponentApp<A: ComponentAppTrait> {
    plugins: HashMap<&'static str, Box<dyn std::any::Any>>,
    assets: Vec<Dir<'static>>,
    app: BoxComponent,
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
        self.app.draw(&mut ctx, (0, 0, width, height), (0, 0, width, height));
            //.into_iter().for_each(|(i, a)| ctx.canvas.draw(i, a))
    }

    async fn on_click(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.width();
        let height = ctx.height();
        let (x, y) = ctx.mouse();
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_click(&mut ctx, (width, height), (x, y));
    }

    async fn on_move(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.width();
        let height = ctx.height();
        let (x, y) = ctx.mouse();
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_move(&mut ctx, (width, height), (x, y));
    }

    async fn on_press(&mut self, _ctx: &mut CanvasContext, _t: String) {}
}

#[macro_export]
macro_rules! create_component_entry_points {
    ($app:ty) => {
        create_canvas_entry_points!(ComponentApp::<$app>);
    };
}

pub trait Drawable: std::fmt::Debug {
    fn size(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32);
    fn draw(&mut self, ctx: &mut ComponentContext, position: Rect, bound: Rect);

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}
    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}

}
//clone_trait_object!(Drawable);

// Text, Color, Opacity, text size, line height, font
#[derive(Clone, Debug)]
pub struct Text(pub String, pub Color, pub u32, pub u32, pub Font);
impl Text {
    pub fn new(text: &str, color: Color, size: u32, line_height: u32, font: Font) -> Self {
        Text(text.to_string(), color, size, line_height, font)
    }

    fn into_inner(self, max_width: u32) -> canvas::Text {
        canvas::Text{text: self.0, color: self.1, width: Some(max_width), size: self.2, line_height: self.3, font: self.4.clone().into_inner()}
    }
}

impl Drawable for Text {
    fn size(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        self.clone().into_inner(max_size.0).size(ctx.canvas)
    }

    fn draw(&mut self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        ctx.canvas.draw(Area((position.0, position.1), Some(bound)), CanvasItem::Text(self.clone().into_inner(position.2)))
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), position: (u32, u32)) {
        if self.1.0 > 0 {self.1 = Color(0, 255, 0, 255)}
        else if self.1.1 > 0 {self.1 = Color(0, 0, 255, 255)}
        else if self.1.2 > 0 {self.1 = Color(255, 0, 0, 255)}
        println!("Text: {:?}", position);
    }
}

pub use canvas::Shape as ShapeType;

//  #[derive(Clone, Copy, Debug)]
//  pub enum ShapeType {
//      Ellipse(u32, (u32, u32)),
//      Rectangle(u32, (u32, u32)),
//      RoundedRectangle(u32, (u32, u32))
//  }

//  impl ShapeType {
//      fn into_inner(self, size: (u32, u32)) -> canvas::Shape {
//          match self {
//              ShapeType::Ellipse(stroke) => canvas::Shape::Ellipse(stroke, size),
//              ShapeType::Rectangle(stroke) => canvas::Shape::Rectangle(stroke, size),
//              ShapeType::RoundedRectangle(stroke, cr) => canvas::Shape::RoundedRectangle(stroke, size, cr),
//          }
//      }
//  }

#[derive(Clone, Debug)]
pub struct Shape(pub ShapeType, pub Color);

impl Drawable for Shape {
    fn size(&mut self, _ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {max_size}

    fn draw(&mut self, ctx: &mut ComponentContext, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Shape(self.0, self.1))
    }
}

#[derive(Clone, Debug)]
pub struct Image(pub ShapeType, pub resources::Image, pub Option<Color>);

impl Drawable for Image {
    fn size(&mut self, _ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {max_size}

    fn draw(&mut self, ctx: &mut ComponentContext, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Image(self.0, self.1.clone().into_inner(), self.2))
    }
}

pub type SizeFn<'a> = Box<dyn FnMut(&mut ComponentContext, (u32, u32)) -> (u32, u32) + 'a>;

pub trait Component: std::fmt::Debug {
    fn build(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> Container;
    fn size(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        self.build(ctx, max_size).size(ctx, max_size)
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}
    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}
}
//clone_trait_object!(Component);

impl<C: Component + ?Sized + 'static> Drawable for C {
    fn size(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        Component::size(self, ctx, max_size)
    }
    fn draw(&mut self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        let max_size = (position.2, position.3);
        self.build(ctx, max_size).draw(ctx, position, bound)
    }

    fn on_click(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: (u32, u32)) {
        self.build(ctx, max_size).on_click(ctx, max_size, position)
    }
    fn on_move(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: (u32, u32)) {
        self.build(ctx, max_size).on_move(ctx, max_size, position)
    }
}

pub trait Layout: std::fmt::Debug {
    fn layout(&self, ctx: &mut ComponentContext, max_size: (u32, u32), items: Vec<SizeFn>) -> Vec<((i32, i32), (u32, u32))>;
    fn size(&self, ctx: &mut ComponentContext, max_size: (u32, u32), items: Vec<SizeFn>) -> (u32, u32) {
        self.layout(ctx, max_size, items).into_iter().fold((0, 0), |old_size, (offset, size)| {
            let size = ((offset.0 + size.0 as i32).max(0) as u32, (offset.1 + size.1 as i32).max(0) as u32);
            (old_size.0.max(size.0), old_size.1.max(size.1))
        })
    }
}

#[derive(Debug)]
pub struct DefaultLayout;
impl Layout for DefaultLayout {
    fn layout(&self, ctx: &mut ComponentContext, max_size: (u32, u32), items: Vec<SizeFn>) -> Vec<((i32, i32), (u32, u32))> {
        items.into_iter().map(|mut i| ((0, 0), i(ctx, max_size))).collect()//Offset::TopLeft, Size::Fit
    }
}

#[derive(Debug)]
pub struct Container<'a>(Box<dyn Layout>, Vec<&'a mut dyn Drawable>);

impl<'a> Container<'a> {

    pub fn new(layout: impl Layout + 'static, items: Vec<&'a mut dyn Drawable>) -> Self {
        Container(Box::new(layout), items.into_iter().rev().collect())
    }

    pub fn get_child(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: (u32, u32)) -> Option<(&mut &'a mut dyn Drawable, (u32, u32))> {
        let items = self.1.iter_mut().map(|c| Box::new(|ctx: &mut ComponentContext, max: (u32, u32)| c.size(ctx, max)) as SizeFn).collect();
        self.0.layout(ctx, max_size, items).into_iter().zip(self.1.iter_mut()).find_map(|((offset, size), child)| {
            if (position.0 as i32) > offset.0 && (position.0 as i32) < offset.0+size.0 as i32 &&
               (position.1 as i32) > offset.1 && (position.1 as i32) < offset.1+size.1 as i32 {
                Some((child, (position.0-offset.0 as u32, position.1-offset.1 as u32)))
            } else {None}
        })
    }
}

impl<'a> Drawable for Container<'a> {
    fn size(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        let items = self.1.iter_mut().map(|c| Box::new(|ctx: &mut ComponentContext, max: (u32, u32)| c.size(ctx, max)) as SizeFn).collect();
        self.0.size(ctx, max_size, items)
    }

    fn draw(&mut self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        let max_size = (position.2, position.3);
        let items = self.1.iter_mut().map(|c| Box::new(|ctx: &mut ComponentContext, max: (u32, u32)| c.size(ctx, max)) as SizeFn).collect();
        self.0.layout(ctx, max_size, items).into_iter().zip(self.1.iter_mut()).rev().for_each(|((offset, size), child)| {
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

    fn on_click(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: (u32, u32)) {
        if let Some((c, p)) = self.get_child(ctx, max_size, position) {
            c.on_click(ctx, max_size, p);
        }
    }

    fn on_move(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: (u32, u32)) {
        if let Some((c, p)) = self.get_child(ctx, max_size, position) {
            c.on_move(ctx, max_size, p);
        }
    }
}

#[macro_export]
macro_rules! container {
    [$($child:expr),* $(,)?] => {{
        Container::new(DefaultLayout, vec![ $(Box::new($child as &mut dyn Drawable)),* ])
    }};
}

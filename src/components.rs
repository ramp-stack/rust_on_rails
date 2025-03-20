use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, Area};
use crate::canvas;

use include_dir::{DirEntry, Dir};

use std::collections::HashMap;

pub mod resources;
use resources::Font;

pub use canvas::Shape as ShapeType;
pub use canvas::Color;

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub x: u32, pub y: u32, pub w: u32, pub h: u32
}

impl Rect {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Rect{x, y, w, h}
    }

    pub fn position(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(self.w, self.h)
    }
}

impl From<Rect> for (u32, u32, u32, u32) {
    fn from(rect: Rect) -> (u32, u32, u32, u32) {
        (rect.x, rect.y, rect.w, rect.h)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Vec2 {
    pub x: u32,
    pub y: u32
}

impl Vec2 {
    pub fn new(x: u32, y: u32) -> Self {
        Vec2{x, y}
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self{
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl From<Vec2> for (u32, u32) {
    fn from(vec: Vec2) -> (u32, u32) {
        (vec.x, vec.y)
    }
}

pub trait Drawable {
    fn draw(&self, ctx: &mut ComponentContext, offset: Vec2, bound: Rect) -> Vec<(Area, CanvasItem)>;
    fn size(&self, ctx: &mut ComponentContext) -> Vec2;
    fn offset(&self) -> Vec2;
}

impl Drawable for CanvasItem {
    fn draw(&self, _ctx: &mut ComponentContext, offset: Vec2, bound: Rect) -> Vec<(Area, CanvasItem)> {
        vec![(Area(offset.into(), Some(bound.into())), self.clone())]
    }
    fn size(&self, ctx: &mut ComponentContext) -> Vec2 {
        let size = CanvasItem::size(self, ctx.canvas);
        Vec2::new(size.0, size.1)
    }
    fn offset(&self) -> Vec2 {Vec2::new(0, 0)}
}

pub struct Component(Vec<Box<dyn Drawable>>, pub Rect);//Children, Bound, ShrinkToFit: bool, Replacing this with Transparent background Container for false

impl Drawable for Component {
    fn draw(&self, ctx: &mut ComponentContext,  offset: Vec2, bound: Rect) -> Vec<(Area, CanvasItem)> {
        let offset = offset+self.1.position();
        let bound = Rect::new(
            bound.x.max(bound.x+self.1.x), bound.y.max(bound.y+self.1.y),//New bound offset
            bound.w.min(self.1.w), bound.h.min(self.1.h)//New bound size
        );

        self.0.iter().flat_map(|c| c.draw(ctx, offset, bound)).collect()
    }

    //Size of an element is Max Size+Offset of its children limited to the Max size
    fn size(&self, ctx: &mut ComponentContext) -> Vec2 {
        let size = self.0.iter().fold(Vec2::new(0, 0), |old_size, c| {
            let size = c.size(ctx);
            let offset = c.offset();
            Vec2::new(old_size.x.max(offset.x+size.x), old_size.y.max(offset.y+size.y))
        });
        Vec2::new(size.x.min(self.1.w), size.y.min(self.1.h))
    }

    fn offset(&self) -> Vec2 {self.1.position()}
}

pub trait ComponentBuilder {
    fn build_children(&self, ctx: &mut ComponentContext, max_size: Vec2) -> Vec<Box<dyn Drawable>>;

    fn build(&self, ctx: &mut ComponentContext, window: Rect) -> Component {
        Component(self.build_children(ctx, window.size()), window)
    }

    fn on_click(&mut self, ctx: &mut ComponentContext, max_size: Vec2, position: Vec2);
    fn on_move(&mut self, ctx: &mut ComponentContext, max_size: Vec2, position: Vec2);
}

impl<T: ComponentBuilder> Drawable for Box<T> {
    fn draw(&self, ctx: &mut ComponentContext, offset: Vec2, bound: Rect) -> Vec<(Area, CanvasItem)> {
        self.build(ctx, bound).draw(ctx, offset, bound)
    }

    //Size of an element is Max Size+Offset of its children limited to the Max size
    fn size(&self, ctx: &mut ComponentContext) -> Vec2 {
        self.build(ctx, Rect::new(0, 0, 0, 0)).size(ctx)
    }

    fn offset(&self) -> Vec2 {Vec2::new(0, 0)}
}

#[derive(Clone)]
pub struct Text(pub &'static str, pub Color, pub Option<u32>, pub u32, pub u32, pub Font);
// Text, Color, Opacity, Optional Width, text size, line height, font

impl ComponentBuilder for Text {
    fn build_children(&self, _ctx: &mut ComponentContext, _max_size: Vec2) -> Vec<Box<dyn Drawable>> {
        vec![Box::new(CanvasItem::Text(canvas::Text::new(self.0, self.1, self.2, self.3, self.4, self.5.clone().into_inner())))]
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}

    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}
}

#[derive(Clone)]
pub struct Shape(pub ShapeType, pub Color);
//  Shape, color, opacity

impl ComponentBuilder for Shape {
    fn build_children(&self, _ctx: &mut ComponentContext, _max_size: Vec2) -> Vec<Box<dyn Drawable>> {
        vec![Box::new(CanvasItem::Shape(self.0, self.1))]
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}

    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}
}

#[derive(Clone)]
pub struct Image(pub ShapeType, pub resources::Image, pub Option<Color>);
// Shape, Image

impl ComponentBuilder for Image {
    fn build_children(&self, _ctx: &mut ComponentContext, _max_size: Vec2) -> Vec<Box<dyn Drawable>> {
        vec![Box::new(CanvasItem::Image(self.0, self.1.clone().into_inner(), self.2))]
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}

    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}
}

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
    fn new(ctx: &mut ComponentContext) -> impl std::future::Future<Output = Box<dyn ComponentBuilder>> where Self: Sized;
}

pub struct ComponentApp<A: ComponentAppTrait> {
    plugins: HashMap<&'static str, Box<dyn std::any::Any>>,
    assets: Vec<Dir<'static>>,
    app: Box<dyn ComponentBuilder>,
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
        self.app.build(&mut ctx, Rect::new(0, 0, width, height))
            .draw(&mut ctx, Vec2::new(0, 0), Rect::new(0, 0, width, height))
            .into_iter().for_each(|(i, a)| ctx.canvas.draw(i, a))
    }

    async fn on_click(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.width();
        let height = ctx.height();
        let (x, y) = ctx.mouse();
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_click(&mut ctx, Vec2::new(width, height), Vec2::new(x, y));
    }

    async fn on_move(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.width();
        let height = ctx.height();
        let (x, y) = ctx.mouse();
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_move(&mut ctx, Vec2::new(width, height), Vec2::new(x, y));
    }

    async fn on_press(&mut self, _ctx: &mut CanvasContext, _t: String) {
      //let mut ctx = ComponentContext::new(self.handles.as_mut().unwrap(), &mut self.assets, ctx);
      //self.app.on_press(&mut ctx, Vec2::new(width, height), t);
    }
}

#[macro_export]
macro_rules! create_component_entry_points {
    ($app:ty) => {
        create_canvas_entry_points!(ComponentApp::<$app>);
    };
}

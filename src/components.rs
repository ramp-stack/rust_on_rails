use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, Area};
use crate::canvas;
use dyn_clone::{clone_trait_object, DynClone};

use include_dir::{DirEntry, Dir};

use std::collections::HashMap;

pub mod resources;
use resources::Font;

pub use canvas::Shape as ShapeType;
pub use canvas::Color;

pub type Bound = (u32, u32, u32, u32);

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
    fn new(ctx: &mut ComponentContext) -> impl std::future::Future<Output = Box<dyn Component>> where Self: Sized;
}

pub struct ComponentApp<A: ComponentAppTrait> {
    plugins: HashMap<&'static str, Box<dyn std::any::Any>>,
    assets: Vec<Dir<'static>>,
    app: Box<dyn Component>,
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
        self.app.draw(&mut ctx, (0, 0), (0, 0, width, height));
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

    async fn on_press(&mut self, _ctx: &mut CanvasContext, _t: String) {
      //let mut ctx = ComponentContext::new(self.handles.as_mut().unwrap(), &mut self.assets, ctx);
      //self.app.on_press(&mut ctx, (width, height), t);
    }
}

#[macro_export]
macro_rules! create_component_entry_points {
    ($app:ty) => {
        create_canvas_entry_points!(ComponentApp::<$app>);
    };
}


pub trait Component: DynClone {
    fn build(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> Vec<((u32, u32), Box<dyn Component>)>;
    fn size(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        self.build(ctx, max_size).iter().fold((0, 0), |old_size, (offset, c)| {
            let max_size = (max_size.0-offset.0, max_size.1-offset.1);
            let size = c.size(ctx, max_size);
            (old_size.0.max(offset.0+size.0), old_size.1.max(offset.1+size.1))
        })
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}
    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}

    fn draw(&self, ctx: &mut ComponentContext, offset: (u32, u32), bound: Bound) {
        let max_size = (bound.2, bound.3);
        for (c_offset, c) in self.build(ctx, max_size) {
            let offset = (offset.0+c_offset.0, offset.1+c_offset.1);
            let max_size = (max_size.0-c_offset.0, max_size.1-c_offset.1);
            let size = c.size(ctx, max_size);
            let bound = (
                bound.0.max(bound.0+c_offset.0), bound.1.max(bound.1+c_offset.1),//New bound offset
                bound.2.min(size.0), bound.3.min(size.1)//New bound size
            );
            c.draw(ctx, offset, bound);
        }
    }
}
clone_trait_object!(Component);

// Text, Color, Opacity, Optional Width, text size, line height, font
#[derive(Clone)]
pub struct Text(pub &'static str, pub Color, pub u32, pub u32, pub Font);
impl Text {
    fn into_inner(self, max_width: u32) -> canvas::Text {
        canvas::Text::new(self.0, self.1, Some(max_width), self.2, self.3, self.4.clone().into_inner())
    }
}

impl Component for Text {
    fn build(&self, _ctx: &mut ComponentContext, _max_size: (u32, u32)) -> Vec<((u32, u32), Box<dyn Component>)> {vec![]}
    fn size(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        self.clone().into_inner(max_size.0).size(ctx.canvas)
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), position: (u32, u32)) {
        println!("Text: {:?}", position);
    }

    fn draw(&self, ctx: &mut ComponentContext, offset: (u32, u32), bound: Bound) {
        ctx.canvas.draw(Area(offset, Some(bound)), CanvasItem::Text(self.clone().into_inner(bound.2)))
    }
}

#[derive(Clone)]
pub struct Shape(pub ShapeType, pub Color);

impl Component for Shape {
    fn build(&self, _ctx: &mut ComponentContext, _max_size: (u32, u32)) -> Vec<((u32, u32), Box<dyn Component>)> {vec![]}
    fn size(&self, _ctx: &mut ComponentContext, _max_size: (u32, u32)) -> (u32, u32) {
        self.0.size()
    }

    fn draw(&self, ctx: &mut ComponentContext, offset: (u32, u32), bound: Bound) {
        ctx.canvas.draw(Area(offset, Some(bound)), CanvasItem::Shape(self.0, self.1))
    }
}

#[derive(Clone)]
pub struct Image(pub ShapeType, pub resources::Image, pub Option<Color>);

impl Component for Image {
    fn build(&self, _ctx: &mut ComponentContext, _max_size: (u32, u32)) -> Vec<((u32, u32), Box<dyn Component>)> {vec![]}
    fn size(&self, _ctx: &mut ComponentContext, _max_size: (u32, u32)) -> (u32, u32) {
        self.0.size()
    }

    fn draw(&self, ctx: &mut ComponentContext, offset: (u32, u32), bound: Bound) {
        ctx.canvas.draw(Area(offset, Some(bound)), CanvasItem::Image(self.0, self.1.clone().into_inner(), self.2))
    }
}

//Exparamental
impl<C: Component + Clone + 'static> Component for Option<C> {
    fn build(&self, _ctx: &mut ComponentContext, _max_size: (u32, u32)) -> Vec<((u32, u32), Box<dyn Component>)> {
        match self {
            Some(c) => vec![((0,0), Box::new(c.clone()))],
            None => vec![]
        }
    }
}

impl<C: Component + 'static> From<C> for Box<dyn Component> {
    fn from(component: C) -> Box<dyn Component> {
        Box::new(component)
    }
}

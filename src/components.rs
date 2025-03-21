use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, Area};
use crate::canvas;
use dyn_clone::{clone_trait_object, DynClone};

use include_dir::{DirEntry, Dir};

use std::collections::HashMap;

pub mod resources;
use resources::Font;

pub use canvas::Color;

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


type BuiltComponent = Vec<Box<dyn Component>>;

pub trait Component: DynClone {
    fn build(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> Container;
    fn size(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        self.build(ctx, max_size).size(ctx, max_size)
      //iter().fold((0, 0), |old_size, c| {
      //    let size = c.size(ctx, max_size);
      //    (old_size.0.max(size.0), old_size.1.max(size.1))
      //})
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}
    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}
}
clone_trait_object!(Component);

pub trait _Component: DynClone {
    fn size(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32);
    fn draw(&self, ctx: &mut ComponentContext, position: Rect, bound: Rect);

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}
    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: (u32, u32)) {}

}
clone_trait_object!(_Component);

impl<C: Component + ?Sized + 'static> _Component for C {
    fn size(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        Component::size(self, ctx, max_size)
    }

    fn draw(&self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        let max_size = (position.2, position.3);
        self.build(ctx, max_size).draw(ctx, position, bound)
      //for c in self.build(ctx, max_size) {
      //    let size = c.size(ctx, max_size);
      //    let bound = (bound.0, bound.1, bound.2.min(size.0), bound.3.min(size.1));//New bound size
      //    //bound.0.max(bound.0+c_offset.0), bound.1.max(bound.1+c_offset.1),//New bound offset
      //    c.draw(ctx, position, bound);
      //}
    }
}

pub trait ComponentTag: _Component + DynClone {}
clone_trait_object!(ComponentTag);

impl<C: _Component + ?Sized + 'static> ComponentTag for C {}


//  impl<C: _Component + ?Sized + 'static> Component for C {
//      fn build(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> Container {
//          Container(Offset::default(), Size::default(), vec![self.clone()])
//      }

//      fn size(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
//          _Component::size(self, ctx, max_size)
//      }
//  }


// Text, Color, Opacity, Optional Width, text size, line height, font
#[derive(Clone)]
pub struct Text(pub &'static str, pub Color, pub u32, pub u32, pub Font);
impl Text {
    fn into_inner(self, max_width: u32) -> canvas::Text {
        canvas::Text::new(self.0, self.1, Some(max_width), self.2, self.3, self.4.clone().into_inner())
    }
}

impl _Component for Text {
    //fn build(&self, _ctx: &mut ComponentContext, _max_size: (u32, u32)) -> BuiltComponent {panic!("Cannot Build Text Component")}
    fn size(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        self.clone().into_inner(max_size.0).size(ctx.canvas)
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), position: (u32, u32)) {
        println!("Text: {:?}", position);
    }

    fn draw(&self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        ctx.canvas.draw(Area((position.0, position.1), Some(bound)), CanvasItem::Text(self.clone().into_inner(position.2)))
    }
}

#[derive(Clone, Copy)]
pub enum ShapeType {//Stroke, (Corner Radius)
    Ellipse(u32),
    Rectangle(u32),
    RoundedRectangle(u32, u32)
}

impl ShapeType {
    fn into_inner(self, size: (u32, u32)) -> canvas::Shape {
        match self {
            ShapeType::Ellipse(stroke) => canvas::Shape::Ellipse(stroke, size),
            ShapeType::Rectangle(stroke) => canvas::Shape::Rectangle(stroke, size),
            ShapeType::RoundedRectangle(stroke, cr) => canvas::Shape::RoundedRectangle(stroke, size, cr),
        }
    }
}

#[derive(Clone)]
pub struct Shape(pub ShapeType, pub Color);

impl _Component for Shape {
    //fn build(&self, _ctx: &mut ComponentContext, _max_size: (u32, u32)) -> BuiltComponent {panic!("Cannot Build Shape Component")}
    fn size(&self, _ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {max_size}

    fn draw(&self, ctx: &mut ComponentContext, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Shape(self.0.into_inner((pos.2, pos.3)), self.1))
    }
}

#[derive(Clone)]
pub struct Image(pub ShapeType, pub resources::Image, pub Option<Color>);

impl _Component for Image {
    //fn build(&self, _ctx: &mut ComponentContext, _max_size: (u32, u32)) -> BuiltComponent {panic!("Cannot Build Image Component")}
    fn size(&self, _ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {max_size}

    fn draw(&self, ctx: &mut ComponentContext, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Image(self.0.into_inner((pos.2, pos.3)), self.1.clone().into_inner(), self.2))
    }
}

#[derive(Clone)]
pub struct Container(pub Offset, pub Size, pub Vec<Box<dyn ComponentTag>>);

impl _Component for Container {
    fn size(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> (u32, u32) {
        self.2.iter().fold((0, 0), |old_size, c| {
            let size = self.1.get(max_size, c.size(ctx, max_size));
            (old_size.0.max(size.0), old_size.1.max(size.1))
        })
    }

    fn draw(&self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        let max_size = (position.2, position.3);
        let max_size = self.size(ctx, max_size);

        for c in &self.2 {
            let min_size = c.size(ctx, max_size);

            let offset = self.0.get(max_size, min_size);

            let position = (
                position.0+offset.0, position.1+offset.1,//Screen Offset Total
                max_size.0, max_size.1//Size of underlaying component
            );
            let bound = (
                bound.0.max(bound.0+offset.0), bound.1.max(bound.1+offset.1),//New bound offset
                bound.2.min(min_size.0), bound.3.min(min_size.1)//New bound size
            );
            c.draw(ctx, position, bound);
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum Size {
    Fit,
    Expand,
    #[default]
    Fill,
    Static(u32, u32),
    Custom(fn((u32, u32), (u32, u32)) -> (u32, u32))
}

impl Size {
    pub fn get(&self, max_size: (u32, u32), c_size: (u32, u32)) -> (u32, u32) {
        match self {
            Self::Fit => (c_size.0.min(max_size.0), c_size.1.min(max_size.1)),
            Self::Expand => c_size,
            Self::Fill => max_size,
            Self::Static(x, y) => (*x, *y),
            Self::Custom(func) => func(max_size, c_size)
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum Offset {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Static(i32, i32),
    #[allow(clippy::type_complexity)]
    Custom(fn((u32, u32), (u32, u32)) -> (i32, i32))
}

impl Offset {
    pub fn get(&self, max_size: (u32, u32), min_size: (u32, u32)) -> (i32, i32) {
        match self {
            Self::TopLeft => (0, 0),
            Self::TopCenter => (((max_size.0 - min_size.0) / 2) as i32, 0),
            Self::TopRight => ((max_size.0 - min_size.0) as i32, 0),
            Self::Left => (0, ((max_size.1 - min_size.1) / 2) as i32),
            Self::Center => (((max_size.0 - min_size.0) / 2) as i32, ((max_size.1 - min_size.1) / 2) as i32),
            Self::Right => ((max_size.0 - min_size.0) as i32, ((max_size.1 - min_size.1) / 2) as i32),
            Self::BottomLeft => (0, (max_size.1 - min_size.1) as i32),
            Self::BottomCenter => (((max_size.0 as i32 - min_size.0 as i32) / 2), (max_size.1 as i32 - min_size.1 as i32)),
            Self::BottomRight => ((max_size.0 as i32 - min_size.0 as i32), (max_size.1 as i32 - min_size.1 as i32)),
            Self::Static(x, y) => (*x, *y),
            Self::Custom(func) => func(max_size, min_size)
        }
    }
}

#[macro_export]
macro_rules! container {
    ($x:expr, $y:expr, [$($child:expr),* $(,)?]) => {{
        Container($x, $y, vec![ $(Box::new($child) as Box<dyn ComponentTag>),* ])
    }};
}

//  impl<C: Component + 'static> From<C> for Box<dyn Component> {
//      fn from(component: C) -> Box<dyn Component> {
//          Box::new(component)
//      }
//  }

//  // impl<C: Component + 'static> From<C> for ((u32, u32), Box<dyn Component>) {
//  //     fn from(component: C) -> ((u32, u32), Box<dyn Component>) {
//  //         ((0, 0), Box::new(component))
//  //     }
//  // }

//  // impl<C: Component + Clone + 'static> Into<((u32, u32), Box<dyn Component>)> for C {
//  //     fn into(component: C) -> ((u32, u32), Box<dyn Component>) {
//  //         ((0, 0), Box::new(component))
//  //     }
//  // }



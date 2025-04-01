use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, Area};
use crate::canvas;

use include_dir::{DirEntry, Dir};

use std::collections::HashMap;
use std::any::TypeId;
use std::fmt::Debug;

mod sizing;
pub use sizing::{MinSize, MaxSize, SizeInfo};

pub mod resources;
use resources::Font;

pub use canvas::Color;

type Rect = (i32, i32, u32, u32);

pub trait Plugin {}

pub struct ComponentContext<'a> {
    plugins: &'a mut HashMap<TypeId, Box<dyn std::any::Any>>,
    assets: &'a mut Vec<Dir<'static>>,
    canvas: &'a mut CanvasContext
}

impl<'a> ComponentContext<'a> {
    pub fn new(
        plugins: &'a mut HashMap<TypeId, Box<dyn std::any::Any>>,
        assets: &'a mut Vec<Dir<'static>>,
        canvas: &'a mut CanvasContext
    ) -> Self {
        ComponentContext{plugins, assets, canvas}
    }

    pub fn configure_plugin<P: Plugin + 'static>(&mut self, plugin: P) {
        self.plugins.insert(TypeId::of::<P>(), Box::new(plugin));
    }

    pub fn get<P: Plugin + 'static>(&mut self) -> &mut P {
        self.plugins.get_mut(&TypeId::of::<P>())
            .unwrap_or_else(|| panic!("Plugin Not Configured: {:?}", std::any::type_name::<P>()))
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
    plugins: HashMap<TypeId, Box<dyn std::any::Any>>,
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
        let size = self.app.size(&mut ctx).get((width, height));
        self.app.draw(&mut ctx, (0, 0, size.0, size.1), (0, 0, width, height));
    }

    async fn on_click(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.width();
        let height = ctx.height();
        let (x, y) = ctx.mouse();
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        let size = self.app.size(&mut ctx).get((width, height));
        let pos = Some((x, y)).filter(|(x, y)| *x < size.0 && *y < size.1);
        self.app.on_click(&mut ctx, size, pos);
    }

    async fn on_move(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.width();
        let height = ctx.height();
        let (x, y) = ctx.mouse();
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        let size = self.app.size(&mut ctx).get((width, height));
        let pos = Some((x, y)).filter(|(x, y)| *x < size.0 && *y < size.1);
        self.app.on_move(&mut ctx, size, pos);
    }

    async fn on_press(&mut self, ctx: &mut CanvasContext, text: String) {
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_press(&mut ctx, text);
    }
}

#[macro_export]
macro_rules! create_component_entry_points {
    ($app:ty) => {
        create_canvas_entry_points!(ComponentApp::<$app>);
    };
}

pub trait Drawable: Debug {
    fn size(&self, ctx: &mut ComponentContext) -> SizeInfo;

    fn draw(&mut self, _ctx: &mut ComponentContext, position: Rect, bound: Rect);
    fn on_tick(&mut self, _ctx: &mut ComponentContext) {}
    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: Option<(u32, u32)>) {}
    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), _position: Option<(u32, u32)>) {}
    fn on_press(&mut self, _ctx: &mut ComponentContext, _text: String) {}
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
    fn size(&self, ctx: &mut ComponentContext) -> SizeInfo {
        SizeInfo::fixed(self.clone().into_inner().size(ctx.canvas))
    }

    fn draw(&mut self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        ctx.canvas.draw(Area((position.0, position.1), Some(bound)), CanvasItem::Text(self.clone().into_inner()))
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: (u32, u32), position: Option<(u32, u32)>) {
        if position.is_some() {
            if self.1.0 > 0 {self.1 = Color(0, 255, 0, 255)}
            else if self.1.1 > 0 {self.1 = Color(0, 0, 255, 255)}
            else if self.1.2 > 0 {self.1 = Color(255, 0, 0, 255)}
        }
    }
}

pub use canvas::Shape as ShapeType;

#[derive(Clone, Copy, Debug)]
pub struct Shape(pub ShapeType, pub Color);

impl Drawable for Shape {
    fn size(&self, _ctx: &mut ComponentContext) -> SizeInfo {SizeInfo::fixed(self.0.size())}

    fn draw(&mut self, ctx: &mut ComponentContext, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Shape(self.0, self.1))
    }
}

#[derive(Clone, Debug)]
pub struct Image(pub ShapeType, pub resources::Image, pub Option<Color>);

impl Drawable for Image {
    fn size(&self, _ctx: &mut ComponentContext) -> SizeInfo {SizeInfo::fixed(self.0.size())}

    fn draw(&mut self, ctx: &mut ComponentContext, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Image(self.0, self.1.clone().into_inner(), self.2))
    }
}

pub trait Events: Debug {
    fn on_resize(&mut self, _ctx: &mut ComponentContext, _size: (u32, u32)) {}
    fn on_tick(&mut self, _ctx: &mut ComponentContext) {}
    fn on_click(&mut self, _ctx: &mut ComponentContext, _position: Option<(u32, u32)>) -> bool {true}
    fn on_move(&mut self, _ctx: &mut ComponentContext, _position: Option<(u32, u32)>) -> bool {true}
    fn on_press(&mut self, _ctx: &mut ComponentContext, _text: String) -> bool {true}
}

pub trait Component: Events + Debug {
    fn children_mut(&mut self) -> Vec<&mut dyn Drawable>;
    fn children(&self) -> Vec<&dyn Drawable>;
    fn layout(&self) -> &dyn Layout;
}

trait _Component: Component {
    fn build(&self, ctx: &mut ComponentContext, max_size: (u32, u32)) -> Vec<((i32, i32), (u32, u32))> {
        let sizes = self.children().into_iter().map(|i| i.size(ctx)).collect();
        self.layout().build(ctx, max_size, sizes)
    }

    fn size(&self, ctx: &mut ComponentContext) -> SizeInfo {
        let sizes = self.children().into_iter().map(|i| i.size(ctx)).collect();
        self.layout().size(ctx, sizes)
    }

    fn pass_event(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: Option<(u32, u32)>, on_click: bool) {
        let mut passed = false;
        self.build(ctx, max_size).into_iter().zip(self.children_mut()).rev().for_each(|((offset, size), child)| {//Reverse to click on the top most element
            let position = position.and_then(|position| (!passed).then(|| (
                (position.0 as i32) > offset.0 &&
                 (position.0 as i32) < offset.0+size.0 as i32 &&
                 (position.1 as i32) > offset.1 &&
                 (position.1 as i32) < offset.1+size.1 as i32
                ).then(|| {
                    passed = true;
                    (position.0-offset.0 as u32, position.1-offset.1 as u32)
            })).flatten());
            if on_click { child.on_click(ctx, size, position); } else { child.on_move(ctx, size, position); }
        });
    }
}
impl<C: Component + ?Sized> _Component for C {}

impl<C: _Component + ?Sized + 'static> Drawable for C {
    fn draw(&mut self, ctx: &mut ComponentContext, position: Rect, bound: Rect) {
        let size = (position.2, position.3);
        Events::on_resize(self, ctx, size);
        self.build(ctx, size).into_iter().zip(self.children_mut()).for_each(|((offset, size), child)| {
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

    fn size(&self, ctx: &mut ComponentContext) -> SizeInfo {_Component::size(self, ctx)}

    fn on_tick(&mut self, ctx: &mut ComponentContext) {
        Events::on_tick(self, ctx);
        self.children_mut().into_iter().for_each(|c| c.on_tick(ctx));
    }

    fn on_click(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: Option<(u32, u32)>) {
        let position = Events::on_click(self, ctx, position).then_some(position).flatten();
        self.pass_event(ctx, max_size, position, true)
    }
    fn on_move(&mut self, ctx: &mut ComponentContext, max_size: (u32, u32), position: Option<(u32, u32)>) {
        let position = Events::on_move(self, ctx, position).then_some(position).flatten();
        self.pass_event(ctx, max_size, position, false)
    }

    fn on_press(&mut self, ctx: &mut ComponentContext, text: String) {
        if Events::on_press(self, ctx, text.clone()) {
            self.children_mut().into_iter().for_each(|c| c.on_press(ctx, text.clone()));
        }
    }
}

pub trait Layout: Debug {
    fn build(&self, ctx: &mut ComponentContext, max_size: (u32, u32), items: Vec<SizeInfo>) -> Vec<((i32, i32), (u32, u32))>;
    fn size(&self, ctx: &mut ComponentContext, items: Vec<SizeInfo>) -> SizeInfo;
}

#[derive(Clone, Debug)]
pub struct DefaultLayout;//Stack((Offset::Start, Offset::Start), (Size::Fit, Size::Fit))
impl Layout for DefaultLayout {
    fn build(&self, _ctx: &mut ComponentContext, size: (u32, u32), items: Vec<SizeInfo>) -> Vec<((i32, i32), (u32, u32))> {
        items.into_iter().map(|i| ((0, 0), i.get(size))).collect()
    }

    fn size(&self, _ctx: &mut ComponentContext, items: Vec<SizeInfo>) -> SizeInfo {
        items.into_iter().reduce(|s, i| s.max(i)).unwrap_or_default()
    }
}

use crate::canvas;
use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, Area};
pub use crate::canvas::{MouseState, KeyboardEvent, KeyboardState};

use include_dir::{DirEntry, Dir};

use std::collections::HashMap;
use std::any::TypeId;
use std::fmt::Debug;

//mod sizing;
//pub use sizing::{MinSize, MaxSize, SizeInfo};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SizeInfo {
    min_width: u32,
    min_height: u32,
    max_width: u32,
    max_height: u32
}

impl SizeInfo {
    pub fn min_width(&self) -> u32 {self.min_width}
    pub fn min_height(&self) -> u32 {self.min_height}
    pub fn max_width(&self) -> u32 {self.max_width}
    pub fn max_height(&self) -> u32 {self.max_height}

    pub fn new(min_width: u32, min_height: u32, max_width: u32, max_height: u32) -> Self {
        if min_width > max_width {panic!("Min Width was Greater Than Max Width");}
        if min_height > max_height {panic!("Min Height was Greater Than Max Height");}
        SizeInfo{min_width, min_height, max_width, max_height}
    }

    pub fn fixed(size: (u32, u32)) -> Self {
        SizeInfo {
            min_width: size.0,
            min_height: size.1,
            max_width: size.0,
            max_height: size.1
        }
    }

    pub fn get(&self, size: (u32, u32)) -> (u32, u32) {
        (
            self.max_width.min(self.min_width.max(size.0)),
            self.max_height.min(self.min_height.max(size.1))
        )
    }

    pub fn add(&self, w: u32, h: u32) -> SizeInfo {
        self.add_width(w).add_height(h)
    }

    pub fn add_width(&self, w: u32) -> SizeInfo {
        SizeInfo::new(self.min_width.saturating_add(w), self.min_height, self.max_width.saturating_add(w), self.max_height)
    }

    pub fn add_height(&self, h: u32) -> SizeInfo {
        SizeInfo::new(self.min_width, self.min_height.saturating_add(h), self.max_width, self.max_height.saturating_add(h))
    }
}

pub mod resources;
use resources::Font;

pub use canvas::Color;

#[derive(Default, Debug, Clone)]
pub struct SizeBranch(pub SizeInfo, Vec<SizeBranch>);

impl SizeBranch {
    pub fn sizes(&self) -> Vec<SizeInfo> {
        self.1.iter().map(|i| i.0).collect()
    }
}

type Rect = (i32, i32, u32, u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub position: Option<(u32, u32)>,
    pub state: MouseState
}

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
    screen: (u32, u32),
    size: SizeBranch,
    _p: std::marker::PhantomData<A>
}

impl<A: ComponentAppTrait> CanvasAppTrait for ComponentApp<A> {
    async fn new(ctx: &mut CanvasContext, width: u32, height: u32) -> Self {
        let mut plugins = HashMap::new();
        let mut assets = Vec::new();
        let mut ctx = ComponentContext::new(&mut plugins, &mut assets, ctx);
        let app = A::new(&mut ctx).await;
        let size = app.size(&mut ctx);
        ComponentApp{plugins, assets, app, screen: (width, height), size, _p: std::marker::PhantomData::<A>}
    }

    async fn on_resize(&mut self, _ctx: &mut CanvasContext, width: u32, height: u32) {
        self.screen = (width, height);
    }

    async fn on_tick(&mut self, ctx: &mut CanvasContext) {
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_tick(&mut ctx);
        self.size = self.app.size(&mut ctx);
        let size = self.size.0.get(self.screen);
        self.app.draw(&mut ctx, self.size.clone(), (0, 0, size.0, size.1), (0, 0, self.screen.0, self.screen.1));
    }

    async fn on_mouse(&mut self, ctx: &mut CanvasContext, event: canvas::MouseEvent) {
        let (x, y) = event.position;
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        let size = self.size.0.get(self.screen);
        let position = Some((x, y)).filter(|(x, y)| *x < size.0 && *y < size.1);
        let event = MouseEvent{position, state: event.state};
        self.app.on_mouse(&mut ctx, self.size.clone(), size, event);
    }

    async fn on_keyboard(&mut self, ctx: &mut CanvasContext, event: KeyboardEvent) {
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, ctx);
        self.app.on_keyboard(&mut ctx, event);
    }
}

#[macro_export]
macro_rules! create_component_entry_points {
    ($app:ty) => {
        create_canvas_entry_points!(ComponentApp::<$app>);
    };
}

#[allow(private_bounds)]
pub trait Drawable: _Drawable + Debug {}
impl<D: _Drawable + ?Sized> Drawable for D {}

trait _Drawable: Debug {
    fn size(&self, ctx: &mut ComponentContext) -> SizeBranch;

    fn draw(&mut self, ctx: &mut ComponentContext, size_info: SizeBranch, position: Rect, bound: Rect);
    fn on_tick(&mut self, _ctx: &mut ComponentContext) {}
    fn on_mouse(&mut self, _ctx: &mut ComponentContext, _size_info: SizeBranch, _max_size: (u32, u32), _event: MouseEvent) {}
    fn on_keyboard(&mut self, _ctx: &mut ComponentContext, _event: KeyboardEvent) {}
}

// Text, Color, Opacity, text size, line height, font
#[derive(Clone, Debug)]
pub struct Text(pub String, pub Color, pub Option<u32>, pub u32, pub u32, pub Font);
// text, color, max_width, font_size, line_height, font
impl Text {
    pub fn new(text: &str, color: Color, width: Option<u32>, size: u32, line_height: u32, font: Font) -> Self {
        Text(text.to_string(), color, width, size, line_height, font)
    }

    fn into_inner(self) -> canvas::Text {
        canvas::Text{text: self.0, color: self.1, width: self.2, size: self.3, line_height: self.4, font: self.5.clone().into_inner()}
    }
}

impl _Drawable for Text {
    fn size(&self, ctx: &mut ComponentContext) -> SizeBranch {
        SizeBranch(SizeInfo::fixed(self.clone().into_inner().size(ctx.canvas)), vec![])
    }

    fn draw(&mut self, ctx: &mut ComponentContext, _size_info: SizeBranch, position: Rect, bound: Rect) {
        ctx.canvas.draw(Area((position.0, position.1), Some(bound)), CanvasItem::Text(self.clone().into_inner()))
    }

    fn on_mouse(&mut self, _ctx: &mut ComponentContext, _size_info: SizeBranch, _max_size: (u32, u32), event: MouseEvent) {
        if event.state == MouseState::Pressed && event.position.is_some() {
            if self.1.0 > 0 {self.1 = Color(0, 255, 0, 255)}
            else if self.1.1 > 0 {self.1 = Color(0, 0, 255, 255)}
            else if self.1.2 > 0 {self.1 = Color(255, 0, 0, 255)}
        }
    }
}

impl Text {
    pub fn value(&mut self) -> &mut String { &mut self.0 }
    pub fn color(&mut self) -> &mut Color { &mut self.1 }
}

pub use canvas::Shape as ShapeType;

#[derive(Clone, Copy, Debug)]
pub struct Shape(pub ShapeType, pub Color);
// shape, color
impl _Drawable for Shape {
    fn size(&self, _ctx: &mut ComponentContext) -> SizeBranch {SizeBranch(SizeInfo::fixed(self.0.size()), vec![])}

    fn draw(&mut self, ctx: &mut ComponentContext, _size_info: SizeBranch, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Shape(self.0, self.1))
    }
}

impl Shape {
    pub fn color(&mut self) -> &mut Color { &mut self.1 }
}

#[derive(Clone, Debug)]
pub struct Image(pub ShapeType, pub resources::Image, pub Option<Color>);
// shape, image, color
impl _Drawable for Image {
    fn size(&self, _ctx: &mut ComponentContext) -> SizeBranch {SizeBranch(SizeInfo::fixed(self.0.size()), vec![])}

    fn draw(&mut self, ctx: &mut ComponentContext, _size_info: SizeBranch, pos: Rect, bound: Rect) {
        ctx.canvas.draw(Area((pos.0, pos.1), Some(bound)), CanvasItem::Image(self.0, self.1.clone().into_inner(), self.2))
    }
}

impl Image {
    pub fn color(&mut self) -> &mut Option<Color> { &mut self.2 }
    pub fn image(&mut self) -> &mut resources::Image { &mut self.1 }
}


pub trait Events: Debug {
    fn on_resize(&mut self, _ctx: &mut ComponentContext, _size: (u32, u32)) {}
    fn on_tick(&mut self, _ctx: &mut ComponentContext) {}
    fn on_mouse(&mut self, _ctx: &mut ComponentContext, _event: MouseEvent) -> bool {true}
    fn on_keyboard(&mut self, _ctx: &mut ComponentContext, _event: KeyboardEvent) -> bool {true}
}

pub trait Component: Events + Debug {
    fn children_mut(&mut self) -> Vec<&mut dyn Drawable>;
    fn children(&self) -> Vec<&dyn Drawable>;
    fn layout(&self) -> &dyn Layout;
}

trait _Component: Component {
    fn build(&self, ctx: &mut ComponentContext, sizes: Vec<SizeInfo>, size: (u32, u32)) -> Vec<((i32, i32), (u32, u32))> {
        self.layout().build(ctx, size, sizes)
    }

    fn pass_mouse(&mut self, ctx: &mut ComponentContext, size_info: SizeBranch, size: (u32, u32), event: MouseEvent) {
        let mut passed = false;
        self.build(ctx, size_info.sizes(), size).into_iter().zip(self.children_mut()).zip(size_info.1).rev().for_each(|(((offset, size), child), branch)| {//Reverse to click on the top most element
            let mut event = event.clone();
            event.position = event.position.and_then(|position| (!passed).then(|| (
                (position.0 as i32) > offset.0 &&
                 (position.0 as i32) < offset.0+size.0 as i32 &&
                 (position.1 as i32) > offset.1 &&
                 (position.1 as i32) < offset.1+size.1 as i32
                ).then(|| {
                    passed = true;
                    ((position.0 as i32 -offset.0) as u32, (position.1 as i32 - offset.1) as u32)
            })).flatten());
            child.on_mouse(ctx, branch, size, event);
        });
    }
}
impl<C: Component + ?Sized> _Component for C {}

impl<C: _Component + ?Sized + 'static> _Drawable for C {
    fn draw(&mut self, ctx: &mut ComponentContext, size_info: SizeBranch, position: Rect, bound: Rect) {
        let size = (position.2, position.3);
        Events::on_resize(self, ctx, size);
        self.build(ctx, size_info.sizes(), size).into_iter().zip(self.children_mut()).zip(size_info.1).for_each(|(((offset, size), child), branch)| {
            let position = (
                position.0+offset.0, position.1+offset.1,//Screen Offset Total
                size.0, size.1//Size of underlaying component
            );

            let bound = (
                bound.0.max(position.0), bound.1.max(position.1),//New bound offset
                bound.2.min((offset.0 + size.0 as i32).max(0) as u32), bound.3.min((offset.1 + size.1 as i32).max(0) as u32)//New bound size
            );

            if bound.2 != 0 && bound.3 != 0 {
                child.draw(ctx, branch, position, bound);
            }
        })
    }

    fn size(&self, ctx: &mut ComponentContext) -> SizeBranch {
        let (sizes, branches): (Vec<_>, Vec<_>) = self.children().into_iter().map(|i| {
            let size = i.size(ctx);
            (size.0, size.1)
        }).unzip();
        SizeBranch(self.layout().size(ctx, sizes.clone()), sizes.into_iter().zip(branches).map(|(size, branch)| SizeBranch(size, branch)).collect())
        //_Component::size(self, ctx)
    }

    fn on_tick(&mut self, ctx: &mut ComponentContext) {
        Events::on_tick(self, ctx);
        self.children_mut().into_iter().for_each(|c| c.on_tick(ctx));
    }

    fn on_mouse(&mut self, ctx: &mut ComponentContext, size_info: SizeBranch, size: (u32, u32), mut event: MouseEvent) {
        event.position = Events::on_mouse(self, ctx, event).then_some(event.position).flatten();
        self.pass_mouse(ctx, size_info, size, event)
    }

    fn on_keyboard(&mut self, ctx: &mut ComponentContext, event: KeyboardEvent) {
        if Events::on_keyboard(self, ctx, event.clone()) {
            self.children_mut().into_iter().for_each(|c| c.on_keyboard(ctx, event.clone()));
        }
    }
}

pub trait Layout: Debug {
    fn build(&self, ctx: &mut ComponentContext, max_size: (u32, u32), items: Vec<SizeInfo>) -> Vec<((i32, i32), (u32, u32))>;
    fn size(&self, ctx: &mut ComponentContext, items: Vec<SizeInfo>) -> SizeInfo;
}

#[derive(Clone, Debug)]
pub struct DefaultLayout(Vec<(i32, i32)>);
impl Layout for DefaultLayout {
    fn build(&self, _ctx: &mut ComponentContext, size: (u32, u32), items: Vec<SizeInfo>) -> Vec<((i32, i32), (u32, u32))> {
        items.into_iter().zip(self.0.clone()).map(|(i, o)| (o, i.get(size))).collect()
    }

    fn size(&self, _ctx: &mut ComponentContext, items: Vec<SizeInfo>) -> SizeInfo {
        items.into_iter().reduce(|s, i| s.max(i)).unwrap_or_default()
    }
}

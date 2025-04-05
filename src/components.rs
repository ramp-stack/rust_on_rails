use crate::canvas;
use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, Area};
pub use crate::canvas::{Field};

use include_dir::{DirEntry, Dir};

use crate::state::State;

use std::collections::HashMap;
use std::any::TypeId;
use std::fmt::Debug;

pub mod resources;
use resources::Font;

mod events;
pub use events::{
    Events, Event, TickEvent, ResizeEvent, MouseEvent, MouseState,
    KeyboardEvent, KeyboardState, NamedKey, Key
};

mod sizing;
pub use sizing::{Layout, SizeInfo};

pub use canvas::Color;

#[derive(Default, Debug, Clone)]
pub struct SizeBranch(pub SizeInfo, Vec<SizeBranch>);

#[derive(Default, Debug, Clone)]
pub struct BuiltBranch(pub (u32, u32), Vec<((i32, i32), BuiltBranch)>);

type Rect = (i32, i32, u32, u32);

pub trait Plugin {}

pub struct ComponentContext<'a> {
    plugins: &'a mut HashMap<TypeId, Box<dyn std::any::Any>>,
    assets: &'a mut Vec<Dir<'static>>,
    events: &'a mut Vec<Box<dyn Event>>,
    canvas: &'a mut CanvasContext,
}

impl<'a> ComponentContext<'a> {
    pub fn new(
        plugins: &'a mut HashMap<TypeId, Box<dyn std::any::Any>>,
        assets: &'a mut Vec<Dir<'static>>,
        events: &'a mut Vec<Box<dyn Event>>,
        canvas: &'a mut CanvasContext,
    ) -> Self {
        ComponentContext{plugins, assets, canvas, events}
    }

    pub fn configure_plugin<P: Plugin + 'static>(&mut self, plugin: P) {
        self.plugins.insert(TypeId::of::<P>(), Box::new(plugin));
    }

    pub fn trigger_event(&mut self, event: impl Event) {
        self.events.push(Box::new(event));
    }

    pub fn get<P: Plugin + 'static>(&mut self) -> &mut P {
        self.plugins.get_mut(&TypeId::of::<P>())
            .unwrap_or_else(|| panic!("Plugin Not Configured: {:?}", std::any::type_name::<P>()))
            .downcast_mut().unwrap()
    }

    pub fn state(&self) -> &State {self.canvas.state()}

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
    fn root(ctx: &mut ComponentContext) -> Box<dyn Drawable> where Self: Sized;
}

pub struct ComponentApp<A: ComponentAppTrait> {
    plugins: HashMap<TypeId, Box<dyn std::any::Any>>,
    assets: Vec<Dir<'static>>,
    app: Box<dyn Drawable>,
    screen: (u32, u32),
    built: BuiltBranch,
    events: Vec<Box<dyn Event>>,
    _p: std::marker::PhantomData<A>
}

impl<A: ComponentAppTrait> CanvasAppTrait for ComponentApp<A> {
    fn new(ctx: &mut CanvasContext, width: u32, height: u32) -> Self {
        let mut plugins = HashMap::new();
        let mut assets = Vec::new();
        let mut events = Vec::new();
        let mut ctx = ComponentContext::new(&mut plugins, &mut assets, &mut events, ctx);
        let app = A::root(&mut ctx);
        let size = app.size(&mut ctx);
        let built = app.build(&mut ctx, (width, height), size);
        ComponentApp{plugins, assets, app, screen: (width, height), built, events: Vec::new(), _p: std::marker::PhantomData::<A>}
    }

    fn on_resize(&mut self, _ctx: &mut CanvasContext, width: u32, height: u32) {
        self.screen = (width, height);
    }

    fn on_tick(&mut self, ctx: &mut CanvasContext) {
        let events = self.events.drain(..).collect::<Vec<_>>(); //Events that are triggered will get run on the next tick
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, &mut self.events, ctx);
        self.app.event(&mut ctx, self.built.clone(), Box::new(TickEvent));
        let size = self.app.size(&mut ctx);
        self.built = self.app.build(&mut ctx, self.screen, size);
        self.app.event(&mut ctx, self.built.clone(), Box::new(ResizeEvent(self.built.0)));
        events.into_iter().for_each(|event|
            if let Some(event) = event.pass(&mut ctx, vec![((0, 0), self.built.0)]).remove(0) {
                self.app.event(&mut ctx, self.built.clone(), event)
            }
        );
        self.app.draw(&mut ctx, self.built.clone(), (0, 0), (0, 0, self.screen.0, self.screen.1));
    }

    fn on_mouse(&mut self, _ctx: &mut CanvasContext, event: canvas::MouseEvent) {
        self.events.push(Box::new(MouseEvent{position: Some(event.position), state: event.state}));
    }

    fn on_keyboard(&mut self, _ctx: &mut CanvasContext, event: KeyboardEvent) {
        self.events.push(Box::new(event));
    }
}

#[macro_export]
macro_rules! create_component_entry_points {
    ($app:ty) => {
        create_canvas_entry_points!(ComponentApp::<$app>);
    };
}

#[allow(private_bounds)]
pub trait Drawable: _Drawable + Debug {
    fn name(&self) -> String;
}
impl<D: _Drawable + ?Sized> Drawable for D {
    fn name(&self) -> String {_Drawable::name(self)}
}

trait _Drawable: Debug {
    fn size(&self, ctx: &mut ComponentContext) -> SizeBranch;
    fn build(&self, _ctx: &mut ComponentContext, size: (u32, u32), size_info: SizeBranch) -> BuiltBranch {
        BuiltBranch(size_info.0.get(size), vec![])
    }
    fn draw(&mut self, ctx: &mut ComponentContext, built: BuiltBranch, offset: (i32, i32), bound: Rect);

    fn name(&self) -> String {std::any::type_name_of_val(self).to_string()}

    fn event(&mut self, _ctx: &mut ComponentContext, _built: BuiltBranch, _event: Box<dyn Event>) {}
}

#[derive(Clone, Debug)]
pub struct Text(pub String, pub Color, pub Option<u32>, pub u32, pub u32, pub Font);
// text, color, max_width, font_size, line_height, font
impl Text {
    pub fn new(text: &str, color: Color, width: Option<u32>, size: u32, line_height: u32, font: Font) -> Self {
        Text(text.to_string(), color, width, size, line_height, font)
    }

    pub fn value(&mut self) -> &mut String { &mut self.0 }//TODO: Turn these fields into named fields
    pub fn color(&mut self) -> &mut Color { &mut self.1 }

    fn into_inner(self) -> canvas::Text {
        canvas::Text{text: self.0, color: self.1, width: self.2, size: self.3, line_height: self.4, font: self.5.clone().into_inner()}
    }
}

impl _Drawable for Text {
    fn size(&self, ctx: &mut ComponentContext) -> SizeBranch {
        SizeBranch(SizeInfo::fixed(self.clone().into_inner().size(ctx.canvas)), vec![])
    }

    fn draw(&mut self, ctx: &mut ComponentContext, _built: BuiltBranch, offset: (i32, i32), bound: Rect) {
        ctx.canvas.draw(Area(offset, Some(bound)), CanvasItem::Text(self.clone().into_inner()))
    }

    fn event(&mut self, _ctx: &mut ComponentContext, _built: BuiltBranch, event: Box<dyn Event>) {
        if let Ok(event) = event.downcast::<MouseEvent>() {
            if event.state == MouseState::Pressed && event.position.is_some() {
                if self.1.0 > 0 {self.1 = Color(0, 255, 0, 255)}
                else if self.1.1 > 0 {self.1 = Color(0, 0, 255, 255)}
                else if self.1.2 > 0 {self.1 = Color(255, 0, 0, 255)}
            }
        }
    }
}

pub use canvas::Shape as ShapeType;

#[derive(Clone, Copy, Debug)]
pub struct Shape(pub ShapeType, pub Color);
// shape, color
impl _Drawable for Shape {
    fn size(&self, _ctx: &mut ComponentContext) -> SizeBranch {SizeBranch(SizeInfo::fixed(self.0.size()), vec![])}

    fn draw(&mut self, ctx: &mut ComponentContext, _built: BuiltBranch, offset: (i32, i32), bound: Rect) {//TODO: use built.0 as the size of the shape?
        ctx.canvas.draw(Area(offset, Some(bound)), CanvasItem::Shape(self.0, self.1))
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

    fn draw(&mut self, ctx: &mut ComponentContext, _built: BuiltBranch, offset: (i32, i32), bound: Rect) {
        ctx.canvas.draw(Area(offset, Some(bound)), CanvasItem::Image(self.0, self.1.clone().into_inner(), self.2))
    }
}

impl Image {
    pub fn color(&mut self) -> &mut Option<Color> { &mut self.2 }
    pub fn image(&mut self) -> &mut resources::Image { &mut self.1 }
}

pub trait Component: Debug {
    fn children_mut(&mut self) -> Vec<&mut dyn Drawable>;
    fn children(&self) -> Vec<&dyn Drawable>;
    fn layout(&self) -> &dyn Layout;
}

impl<C: Component + ?Sized + 'static + Events> _Drawable for C {
    fn size(&self, ctx: &mut ComponentContext) -> SizeBranch {
        let requests = self.children().into_iter().map(|i| i.size(ctx)).collect::<Vec<_>>();
        let info = requests.iter().map(|i| i.0).collect::<Vec<_>>();
        SizeBranch(self.layout().size(ctx, info), requests)
    }

    fn build(&self, ctx: &mut ComponentContext, size: (u32, u32), size_info: SizeBranch) -> BuiltBranch {
        let size = size_info.0.get(size);
        let info = size_info.1.iter().map(|b| b.0).collect::<Vec<_>>();
        BuiltBranch(
            size,
            self.layout()
            .build(ctx, size, info).into_iter()
            .zip(self.children()).zip(size_info.1)
            .map(|(((offset, size), child), branch)| {
                (offset, child.build(ctx, size, branch))
            }).collect()
        )
    }

    fn draw(&mut self, ctx: &mut ComponentContext, built: BuiltBranch, poffset: (i32, i32), bound: Rect) {
        built.1.into_iter().zip(self.children_mut()).for_each(|((offset, branch), child)| {
            let size = branch.0;
            let poffset = (poffset.0+offset.0, poffset.1+offset.1);

            let bound = (
                bound.0.max(poffset.0), bound.1.max(poffset.1),//New bound offset
                bound.2.min((offset.0 + size.0 as i32).max(0) as u32), bound.3.min((offset.1 + size.1 as i32).max(0) as u32)//New bound size
            );

            if bound.2 != 0 && bound.3 != 0 {
                child.draw(ctx, branch, poffset, bound);
            }
        })
    }

    fn event(&mut self, ctx: &mut ComponentContext, built: BuiltBranch, mut event: Box<dyn Event>) {
        if Events::on_event(self, ctx, &mut *event) {
            let children = built.1.iter().map(|(o, branch)| (*o, branch.0)).collect::<Vec<_>>();
            event.pass(ctx, children).into_iter().zip(self.children_mut()).zip(built.1).for_each(
                |((e, child), branch)| if let Some(e) = e {child.event(ctx, branch.1, e);}
            );
        }
    }
}

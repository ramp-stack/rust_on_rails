use crate::canvas;
use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, State};
pub use crate::canvas::{Field};

use include_dir::{DirEntry, Dir};

use std::collections::HashMap;
use std::any::TypeId;
use std::fmt::Debug;

pub mod resources;
use resources::Font;

mod events;
pub use events::{
    Events, Event, TickEvent, MouseEvent, MouseState,
    KeyboardEvent, KeyboardState, NamedKey, Key
};

mod sizing;
pub use sizing::{Layout, SizeRequest, Area};

pub use canvas::Color;

#[derive(Default, Debug, Clone)]
pub struct RequestBranch(pub SizeRequest, Vec<RequestBranch>);

#[derive(Default, Debug, Clone)]
pub struct SizedBranch(pub Size, Vec<(Offset, SizedBranch)>);

type Offset = (i32, i32);
type Rect = (i32, i32, u32, u32);
type Size = (u32, u32);


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
    sized_app: SizedBranch,
    events: Vec<Box<dyn Event>>,
    _p: std::marker::PhantomData<A>
}

impl<A: ComponentAppTrait> CanvasAppTrait for ComponentApp<A> {
    fn new(ctx: &mut CanvasContext, width: u32, height: u32) -> Self {
        let mut plugins = HashMap::new();
        let mut assets = Vec::new();
        let mut events = Vec::new();
        let mut ctx = ComponentContext::new(&mut plugins, &mut assets, &mut events, ctx);
        let mut app = A::root(&mut ctx);
        let size_request = _Drawable::request_size(&*app, &mut ctx);
        let sized_app = app.build(&mut ctx, (width, height), size_request);
        ComponentApp{plugins, assets, app, screen: (width, height), sized_app, events: Vec::new(), _p: std::marker::PhantomData::<A>}
    }

    fn on_resize(&mut self, _ctx: &mut CanvasContext, width: u32, height: u32) {
        self.screen = (width, height);
    }

    fn on_tick(&mut self, ctx: &mut CanvasContext) {
        let events = self.events.drain(..).collect::<Vec<_>>();//Events triggered on this tick will be run on the next tick
        let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, &mut self.events, ctx);
        self.app.event(&mut ctx, self.sized_app.clone(), Box::new(TickEvent));
        events.into_iter().for_each(|event|
            if let Some(event) = event.pass(&mut ctx, vec![((0, 0), self.sized_app.0)]).remove(0) {
                self.app.event(&mut ctx, self.sized_app.clone(), event)
            }
        );

        let size_request = _Drawable::request_size(&*self.app, &mut ctx);
        self.sized_app = self.app.build(&mut ctx, self.screen, size_request);
        self.app.draw(&mut ctx, self.sized_app.clone(), (0, 0), (0, 0, self.screen.0, self.screen.1));
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
    fn request_size(&self, ctx: &mut ComponentContext) -> SizeRequest;
    fn name(&self) -> String;
}
impl<D: _Drawable + ?Sized> Drawable for D {
    fn request_size(&self, ctx: &mut ComponentContext) -> SizeRequest {_Drawable::request_size(self, ctx).0}
    fn name(&self) -> String {_Drawable::name(self)}
}

trait _Drawable: Debug {
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
    pub max_width: Option<u32>,
    pub font_size: u32,
    pub line_height: u32,
    pub font: Font
}

impl Text {
    pub fn new(text: &str, color: Color, max_width: Option<u32>, font_size: u32, line_height: u32, font: Font) -> Self {
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

    fn event(&mut self, _ctx: &mut ComponentContext, _sized: SizedBranch, event: Box<dyn Event>) {
        if let Ok(event) = event.downcast::<MouseEvent>() {
            if event.state == MouseState::Pressed && event.position.is_some() {
                if self.color.0 > 0 && self.color.1 == 0 {self.color = Color(0, 255, 0, 255)}
                else if self.color.0 > 0 && self.color.1 > 0 {self.color = Color(255, 0, 0, 255)}
                else if self.color.1 > 0 {self.color = Color(0, 0, 255, 255)}
                else if self.color.2 > 0 {self.color = Color(255, 255, 255, 255)}
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
                bound.2.min((offset.0 + size.0 as i32).max(0) as u32), bound.3.min((offset.1 + size.1 as i32).max(0) as u32)//New bound size
            );

            if bound.2 != 0 && bound.3 != 0 {
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

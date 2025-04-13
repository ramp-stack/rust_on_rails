use crate::canvas;
use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem};
pub use crate::base::{State, Field, AsyncTasks};

use crate::base::Callback;

use include_dir::{DirEntry, Dir};

use std::collections::HashMap;
use std::time::Duration;
use std::future::Future;
use std::any::TypeId;
use std::fmt::Debug;

pub mod resources;
use resources::Font;

mod events;
pub use events::{
    Events, Event, TickEvent, MouseEvent, MouseState,
    KeyboardEvent, KeyboardState, NamedKey, Key, SmolStr
};

mod sizing;
pub use sizing::{Layout, SizeRequest, Area};

mod drawable;
pub use drawable::*;

pub use canvas::Color;

pub trait Plugin {}

pub struct ComponentContext<'a, 'b> {
    plugins: &'a mut HashMap<TypeId, Box<dyn std::any::Any>>,
    assets: &'a mut Vec<Dir<'static>>,
    events: &'a mut Vec<Box<dyn Event>>,
    canvas: &'a mut CanvasContext<'b>,
}

impl<'a, 'b> ComponentContext<'a, 'b> {
    pub fn new(
        plugins: &'a mut HashMap<TypeId, Box<dyn std::any::Any>>,
        assets: &'a mut Vec<Dir<'static>>,
        events: &'a mut Vec<Box<dyn Event>>,
        canvas: &'a mut CanvasContext<'b>,
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

    pub fn state(&mut self) -> &mut State {self.canvas.state()}

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
    fn register_tasks() -> impl Future<Output = AsyncTasks> where Self: Sized;
    fn root(ctx: &mut ComponentContext) -> impl std::future::Future<Output = Box<dyn Drawable>> where Self: Sized;
}

pub struct ComponentApp<A: ComponentAppTrait> {
    plugins: HashMap<TypeId, Box<dyn std::any::Any>>,
    assets: Vec<Dir<'static>>,
    app: Box<dyn Drawable>,
    screen: (f32, f32),
    sized_app: SizedBranch,
    events: Vec<Box<dyn Event>>,
    _p: std::marker::PhantomData<A>
}

impl<A: ComponentAppTrait> CanvasAppTrait for ComponentApp<A> {
    async fn register_tasks() -> AsyncTasks {A::register_tasks().await}
    async fn new<'a>(ctx: &'a mut CanvasContext<'a>, width: f32, height: f32) -> Self {
        let mut plugins = HashMap::new();
        let mut assets = Vec::new();
        let mut events = Vec::new();
        let mut ctx = ComponentContext::new(&mut plugins, &mut assets, &mut events, ctx);
        let mut app = A::root(&mut ctx).await;
        let size_request = _Drawable::request_size(&*app, &mut ctx);
        let sized_app = app.build(&mut ctx, (width, height), size_request);
        ComponentApp{plugins, assets, app, screen: (width, height), sized_app, events: Vec::new(), _p: std::marker::PhantomData::<A>}
    }

    fn on_event<'a>(&'a mut self, ctx: &'a mut CanvasContext<'a>, event: canvas::WindowEvent) {
        match event {
            canvas::WindowEvent::Resize{width, height} |
            canvas::WindowEvent::Resume{width, height} => {
                self.screen = (width, height);
            },
            canvas::WindowEvent::Mouse{position, state} => {
                self.events.push(Box::new(MouseEvent{position: Some(position), state}));
            },
            canvas::WindowEvent::Keyboard{key, state} => {
                self.events.push(Box::new(KeyboardEvent{key, state}));
            },
            canvas::WindowEvent::Tick => {
                let events = self.events.drain(..).collect::<Vec<_>>();//Events triggered on this tick will be run on the next tick
                let mut ctx = ComponentContext::new(&mut self.plugins, &mut self.assets, &mut self.events, ctx);
                self.app.event(&mut ctx, self.sized_app.clone(), Box::new(TickEvent));
                events.into_iter().for_each(|event|
                    if let Some(event) = event.pass(&mut ctx, vec![((0.0, 0.0), self.sized_app.0)]).remove(0) {
                        self.app.event(&mut ctx, self.sized_app.clone(), event)
                    }
                );

                let size_request = _Drawable::request_size(&*self.app, &mut ctx);
                self.sized_app = self.app.build(&mut ctx, self.screen, size_request);
                self.app.draw(&mut ctx, self.sized_app.clone(), (0.0, 0.0), (0.0, 0.0, self.screen.0, self.screen.1));
            },
            _ => {}
        }
    }
}

#[macro_export]
macro_rules! create_component_entry_points {
    ($app:ty, $bg_app:ty) => {
        create_canvas_entry_points!(ComponentApp::<$app>, $bg_app);
    };
}



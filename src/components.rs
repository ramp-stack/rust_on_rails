use crate::base;

pub use base::renderer::wgpu_canvas::{Canvas, CanvasContext, Color};
pub use include_dir::include_dir as include_assets;
pub use include_dir;
pub use proc::{Component, Plugin};

use base::runtime::{Tasks};
use base::{BaseAppTrait, HeadlessContext};
use base::driver::state::State;
use base::renderer::wgpu_canvas as canvas;
use canvas::Area as CanvasArea;
use canvas::CanvasItem;

use include_dir::{DirEntry, Dir};


use std::collections::HashMap;
use std::future::Future;
use std::time::Instant;
use std::any::TypeId;

mod events;
pub use events::{
    Events, OnEvent, Event, TickEvent, MouseEvent, MouseState,
    KeyboardEvent, KeyboardState, NamedKey, Key, SmolStr
};

pub mod resources;

mod sizing;
pub use sizing::{Layout, SizeRequest, Area};

mod drawable;
pub use drawable::{Component, Text, Image, Shape, RequestBranch, SizedBranch, Drawable, ShapeType};
use drawable::{_Drawable};

pub type Assets = Vec<Dir<'static>>;

pub struct Context<'a> {
    plugins: &'a mut Plugins,
    assets: &'a mut Assets,
    events: &'a mut Events,
    base_context: &'a mut base::Context<Canvas>,
}

impl<'a> Context<'a> {
    pub fn new(
        plugins: &'a mut Plugins,
        assets: &'a mut Assets,
        events: &'a mut Events,
        base_context: &'a mut base::Context<Canvas>
    ) -> Self {
        Context{plugins, assets, events, base_context}
    }
        
    pub fn trigger_event(&mut self, event: impl Event) {
        self.events.push_back(Box::new(event));
    }

    pub fn get<P: Plugin + 'static>(&mut self) -> &mut P {
        self.plugins.get_mut(&TypeId::of::<P>())
            .unwrap_or_else(|| panic!("Plugin Not Configured: {:?}", std::any::type_name::<P>()))
            .downcast_mut().unwrap()
    }

    pub fn state(&mut self) -> &mut State {self.base_context.state()}

    pub fn include_assets(&mut self, dir: Dir<'static>) {
        self.assets.push(dir);
    }

    pub fn add_font(&mut self, font: &[u8]) -> canvas::Font {self.base_context.render_ctx().add_font(font)}
    pub fn add_image(&mut self, image: image::RgbaImage) -> canvas::Image {self.base_context.render_ctx().add_image(image)}
    pub fn load_font(&mut self, file: &str) -> canvas::Font {
        self.load_file(file).map(|b| self.add_font(&b)).expect("Could not find file")
    }

    pub fn load_image(&mut self, file: &str) -> canvas::Image {
        self.load_file(file).map(|b|
            self.add_image(image::load_from_memory(&b).unwrap().into())
        ).expect("Could not find file")
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

pub trait Plugin {
    fn background_tasks(_ctx: &mut HeadlessContext) -> impl Future<Output = Tasks> {async {vec![]}}
    fn new(
        ctx: &mut Context<'_>, h_ctx: &mut HeadlessContext
    ) -> impl Future<Output = (Self, Tasks)> where Self: Sized;
}

pub type Plugins = HashMap<TypeId, Box<dyn std::any::Any>>;

pub trait App {
    fn background_tasks(_ctx: &mut HeadlessContext) -> impl Future<Output = Tasks> {async {vec![]}}

    fn plugins(
        _ctx: &mut Context<'_>, _h_ctx: &mut HeadlessContext
    ) -> impl Future<Output = (Plugins, Tasks)> {async {(HashMap::new(), vec![])}}

    fn new(ctx: &mut Context<'_>) -> impl Future<Output = Box<dyn Drawable>>;
}

pub struct ComponentApp<A: App> {
    plugins: Plugins,
    assets: Assets,
    events: Events,
    app: Box<dyn Drawable>,
    screen: (f32, f32),
    sized_app: SizedBranch,
    _p: std::marker::PhantomData<A>,

    time: Instant
}

impl<A: App> BaseAppTrait<Canvas> for ComponentApp<A> {
    const LOG_LEVEL: log::Level = log::Level::Error;

    async fn background_tasks(ctx: &mut HeadlessContext) -> Tasks {
        A::background_tasks(ctx).await
    }

    async fn new(
        base_ctx: &mut base::Context<Canvas>, h_ctx: &mut HeadlessContext, width: f32, height: f32
    ) -> (Self, Tasks) {
        let mut plugins = HashMap::new();
        let mut assets = Assets::new();
        let mut events = Events::new();
        let mut ctx = Context::new(&mut plugins, &mut assets, &mut events, base_ctx);
        let (mut plugins, tasks) = A::plugins(&mut ctx, h_ctx).await;
        let mut ctx = Context::new(&mut plugins, &mut assets, &mut events, base_ctx);

        let mut app = A::new(&mut ctx).await;
        let size_request = _Drawable::request_size(&*app, &mut ctx);
        let screen = (width, height);
        let sized_app = app.build(&mut ctx, screen, size_request);
        (
            ComponentApp{plugins, assets, events, app, screen, sized_app, _p: std::marker::PhantomData::<A>, time: Instant::now()},
            tasks
        )
    }

    //TODO: Add Pause Resume And Close Events
    //Event Order: Event::Tick => TickEvent, Other Captured/Triggered Events, Draw call
    fn on_event(&mut self, base_ctx: &mut base::Context<Canvas>, event: base::Event) {
        match event {
            base::Event::Resized{width, height} | base::Event::Resumed{width, height} => {
                self.screen = (width, height);
            },
            base::Event::Mouse{position, state} => {
                self.events.push_back(Box::new(MouseEvent{position: Some(position), state}));
            },
            base::Event::Keyboard{key, state} => {
                self.events.push_back(Box::new(KeyboardEvent{key, state}));
            },
            base::Event::Tick => {
                let mut ctx = Context::new(&mut self.plugins, &mut self.assets, &mut self.events, base_ctx);
                self.app.event(&mut ctx, self.sized_app.clone(), Box::new(TickEvent));
                while let Some(event) = self.events.pop_front() {
                    let mut ctx = Context::new(&mut self.plugins, &mut self.assets, &mut self.events, base_ctx);
                    if let Some(event) = event.pass(&mut ctx, vec![((0.0, 0.0), self.sized_app.0)]).remove(0) {
                        self.app.event(&mut ctx, self.sized_app.clone(), event)
                    }
                }

            },
            _ => {}
        }
    }

    fn draw(&mut self, ctx: &mut base::Context<Canvas>) -> Vec<(CanvasArea, CanvasItem)> {
        log::error!("last_frame: {:?}", self.time.elapsed());
        self.time = Instant::now();
        let mut ctx = Context::new(&mut self.plugins, &mut self.assets, &mut self.events, ctx);

        let size_request = _Drawable::request_size(&*self.app, &mut ctx);
        self.sized_app = self.app.build(&mut ctx, self.screen, size_request);
        self.app.draw(&mut ctx, self.sized_app.clone(), (0.0, 0.0), (0.0, 0.0, self.screen.0, self.screen.1))
    }

    async fn close(self) {}
}

#[macro_export]
macro_rules! create_entry_points {
    ($app:ty) => {
        create_base_entry_points!(Canvas, ComponentApp::<$app>);
    };
}

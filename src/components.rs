use crate::base;

pub use include_dir::include_dir as include_assets;
pub use include_dir;
pub use proc::{Component, Plugin};

use base::{BaseAppTrait, HeadlessContext};
use base::driver::runtime::{Tasks};
use base::driver::state::State;
use base::renderer::wgpu_canvas as canvas;
pub use canvas::Canvas;
use canvas::Context as CanvasContext;

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
pub use sizing::{Layout, SizeRequest, DefaultStack, Area};

mod drawable;
pub use drawable::{Component, Text, Font, Span, Cursor, CursorAction, Align, Image, Shape, RequestBranch, SizedBranch, Drawable, ShapeType, Color};
use drawable::{_Drawable};

pub type Assets = Vec<Dir<'static>>;

pub struct Context {
    plugins: Plugins,
    assets: Assets,
    events: Events,
    base_context: base::Context<Canvas>,
}

impl Context {
    pub fn new(base_context: base::Context<Canvas>) -> Self {
        Context{plugins: Plugins::new(), assets: Assets::new(), events: Events::new(), base_context}
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

    pub fn get_clipboard(&mut self) -> String {self.base_context.get_clipboard()}
    pub fn set_clipboard(&mut self, text: String){self.base_context.set_clipboard(text)}

    pub fn add_font(&mut self, font: &[u8]) -> canvas::Font {self.base_context.as_mut().add_font(font)}
    pub fn add_image(&mut self, image: image::RgbaImage) -> canvas::Image {self.base_context.as_mut().add_image(image)}
    pub fn add_svg(&mut self, svg: &[u8], quality: f32) -> canvas::Image {self.base_context.as_mut().add_svg(svg, quality)}
    pub fn load_font(&mut self, file: &str) -> Option<canvas::Font> {
        self.load_file(file).map(|b| self.add_font(&b))
    }

    pub fn load_image(&mut self, file: &str) -> Option<canvas::Image> {
        self.load_file(file).map(|b|
            self.add_image(image::load_from_memory(&b).unwrap().into())
        )
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

    pub fn as_canvas(&mut self) -> &mut CanvasContext {self.as_mut()}
}

impl AsMut<CanvasContext> for Context {
    fn as_mut(&mut self) -> &mut CanvasContext {self.base_context.as_mut()}
}

impl AsMut<wgpu_canvas::FontAtlas> for Context {
    fn as_mut(&mut self) -> &mut wgpu_canvas::FontAtlas {self.base_context.as_mut().as_mut()}
}

pub trait Plugin {
    fn background_tasks(_ctx: &mut HeadlessContext) -> impl Future<Output = Tasks> {async {vec![]}}
    fn new(
        ctx: &mut Context, h_ctx: &mut HeadlessContext
    ) -> impl Future<Output = (Self, Tasks)> where Self: Sized;
}

pub type Plugins = HashMap<TypeId, Box<dyn std::any::Any>>;

pub trait App {
    fn background_tasks(_ctx: &mut HeadlessContext) -> impl Future<Output = Tasks> {async {vec![]}}

    fn plugins(
        _ctx: &mut Context, _h_ctx: &mut HeadlessContext
    ) -> impl Future<Output = (Plugins, Tasks)> {async {(HashMap::new(), vec![])}}

    fn new(ctx: &mut Context) -> impl Future<Output = Box<dyn Drawable>>;
}

pub struct ComponentApp<A: App> {
    ctx: Context,
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
        base_ctx: base::Context<Canvas>, h_ctx: &mut HeadlessContext, width: f32, height: f32
    ) -> (Self, Tasks) {
        let mut ctx = Context::new(base_ctx);
        let (plugins, tasks) = A::plugins(&mut ctx, h_ctx).await;
        ctx.plugins = plugins;
        let mut app = A::new(&mut ctx).await;
        let size_request = _Drawable::request_size(&*app, &mut ctx);
        let screen = (width, height);
        let sized_app = app.build(&mut ctx, screen, size_request);
        (
            ComponentApp{ctx, app, screen, sized_app, _p: std::marker::PhantomData::<A>, time: Instant::now()},
            tasks
        )
    }

    //TODO: Add Pause Resume And Close Events
    //Event Order: Event::Tick => TickEvent, Other Captured/Triggered Events, Draw call
    fn on_event(&mut self, event: canvas::Event) {
        match event {
            canvas::Event::Resized{width, height} | canvas::Event::Resumed{width, height} => {
                self.screen = (width, height);
            },
            canvas::Event::Mouse{position, state} => {
                self.ctx.events.push_back(Box::new(MouseEvent{position: Some(position), state}));
            },
            canvas::Event::Keyboard{key, state} => {
                self.ctx.events.push_back(Box::new(KeyboardEvent{key, state}));
            },
            canvas::Event::Tick => {
                log::error!("last_frame: {:?}", self.time.elapsed());
                self.time = Instant::now();

                self.app.event(&mut self.ctx, self.sized_app.clone(), Box::new(TickEvent));
                while let Some(event) = self.ctx.events.pop_front() {
                    if let Some(event) = event.pass(&mut self.ctx, vec![((0.0, 0.0), self.sized_app.0)]).remove(0) {
                        self.app.event(&mut self.ctx, self.sized_app.clone(), event)
                    }
                }

                let size_request = _Drawable::request_size(&*self.app, &mut self.ctx);
                self.sized_app = self.app.build(&mut self.ctx, self.screen, size_request);
                self.app.draw(&mut self.ctx, self.sized_app.clone(), (0.0, 0.0), (0.0, 0.0, self.screen.0, self.screen.1));
            },
            _ => {}
        }
    }

    async fn close(self) -> base::Context<Canvas> {self.ctx.base_context}

    fn ctx(&mut self) -> &mut base::Context<Canvas> {&mut self.ctx.base_context}
}

#[macro_export]
macro_rules! create_entry_points {
    ($app:ty) => {
        create_base_entry_points!(Canvas, ComponentApp::<$app>);
    };
}

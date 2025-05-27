pub use include_dir;
pub use include_dir::include_dir as include_assets;
pub use proc::{Component, Plugin};

use std::any::TypeId;
use std::collections::HashMap;
use std::future::Future;
use std::time::Instant;

use crate::base;
use base::{BaseAppTrait, HeadlessContext};
use base::driver::runtime::Tasks;
use base::driver::state::State;
use base::driver::share::Share;

use base::renderer::wgpu_canvas as canvas;
pub use canvas::Canvas;
use canvas::Context as CanvasContext;

use include_dir::{Dir, DirEntry};

mod events;
pub use events::{
    Events, OnEvent, Event, TickEvent,
    MouseEvent, MouseState,
    KeyboardEvent, KeyboardState,
    NamedKey, Key, SmolStr,
};

pub mod resources;

mod sizing;
pub use sizing::{Layout, SizeRequest, DefaultStack, Area};

mod drawable;
pub use drawable::{
    Component, Text, Font, Span, Cursor, CursorAction,
    Align, Image, Shape, RequestBranch, SizedBranch,
    Drawable, ShapeType, Color,
};
use drawable::_Drawable;

/// Type alias for a list of directories containing UI assets. (e.g. images, fonts, etc.)
pub type Assets = Vec<Dir<'static>>;

/// The [`Context`] struct encapsulates the global state of an application,
/// including plugin management, asset storage, event dispatching, and
/// access to the rendering and state context provided by the [`base`] crate.
pub struct Context {
    plugins: Plugins,
    assets: Assets,
    events: Events,
    base_context: base::Context<Canvas>,
}

impl Context {
    /// Creates a new [`Context`] from a given [`base::Context<Canvas>`].
    pub fn new(base_context: base::Context<Canvas>) -> Self {
        Context {
            plugins: Plugins::new(),
            assets: Assets::new(),
            events: Events::new(),
            base_context,
        }
    }

    /// Pushes a new event into the event queue.
    ///
    /// # Arguments
    ///
    /// * `event` - Any type implementing the [`Event`] trait.
    pub fn trigger_event(&mut self, event: impl Event) {
        // println!("EVENT TRIGGERED {:?}", event);
        self.events.push_back(Box::new(event));
    }

    /// Retrieves a mutable reference to a registered plugin of type `P`.
    ///
    /// # Panics
    ///
    /// Panics if the plugin has not been registered.
    pub fn get<P: Plugin + 'static>(&mut self) -> &mut P {
        self.plugins.get_mut(&TypeId::of::<P>())
            .unwrap_or_else(|| panic!("Plugin Not Configured: {:?}", std::any::type_name::<P>()))
            .downcast_mut().unwrap()
    }

    /// Returns a mutable reference to the internal [`State`] object.
    pub fn state(&mut self) -> &mut State {
        self.base_context.state()
    }

    /// Includes a static directory of assets into the asset manager.
    ///
    /// # Arguments
    ///
    /// * `dir` - A [`Dir`] representing a directory of embedded assets.
    pub fn include_assets(&mut self, dir: Dir<'static>) {
        self.assets.push(dir);
    }

    /// Retrieves the contents of the system clipboard as a `String`.
    // pub fn get_clipboard(&mut self) -> String {
    //     self.base_context.get_clipboard()
    // }

    /// Sets the contents of the system clipboard.
    ///
    /// # Arguments
    ///
    /// * `text` - A `String` to be copied to the clipboard.
    // pub fn set_clipboard(&mut self, text: String) {
    //     self.base_context.set_clipboard(text)
    // }

    pub fn share(&mut self, text: &str) {
        Share::share(text)
    }

    /// Adds a font from raw bytes and returns a reference to the internal font handle.
    ///
    /// # Arguments
    ///
    /// * `font` - A slice of raw font bytes (e.g., `.ttf` or `.otf`).
    pub fn add_font(&mut self, font: &[u8]) -> canvas::Font {
        self.base_context.as_mut().add_font(font)
    }

    /// Adds a raster image and returns an internal image handle.
    ///
    /// # Arguments
    ///
    /// * `image` - A [`RgbaImage`] to be added to the canvas.
    pub fn add_image(&mut self, image: image::RgbaImage) -> canvas::Image {
        self.base_context.as_mut().add_image(image)
    }

    /// Adds an SVG image with the specified quality.
    ///
    /// # Arguments
    ///
    /// * `svg` - A byte slice containing SVG data.
    /// * `quality` - A `f32` value controlling rasterization quality.
    pub fn add_svg(&mut self, svg: &[u8], quality: f32) -> canvas::Image {
        self.base_context.as_mut().add_svg(svg, quality)
    }

    /// Loads and adds a font from an embedded asset file by path.
    ///
    /// # Arguments
    ///
    /// * `file` - The asset path of the font file.
    ///
    /// # Returns
    ///
    /// An optional [`canvas::Font`] if loading and parsing succeeds.
    pub fn load_font(&mut self, file: &str) -> Option<canvas::Font> {
        self.load_file(file).map(|b| self.add_font(&b))
    }

    /// Loads and adds an image from an embedded asset file by path.
    ///
    /// # Arguments
    ///
    /// * `file` - The asset path of the image file.
    ///
    /// # Returns
    ///
    /// An optional [`canvas::Image`] if loading and decoding succeeds.
    pub fn load_image(&mut self, file: &str) -> Option<canvas::Image> {
        self.load_file(file).map(|b|
            self.add_image(image::load_from_memory(&b).unwrap().into())
        )
    }

    /// Loads a file's raw bytes from the embedded assets.
    ///
    /// # Arguments
    ///
    /// * `file` - The relative path to the asset file.
    ///
    /// # Returns
    ///
    /// An optional `Vec<u8>` containing the file's contents.
    pub fn load_file(&self, file: &str) -> Option<Vec<u8>> {
        self.assets.iter().find_map(|dir|
            dir.find(file).ok().and_then(|mut f|
                f.next().and_then(|f|
                    if let DirEntry::File(f) = f {
                        Some(f.contents().to_vec())
                    } else {
                        None
                    }
                )
            )
        )
    }

    /// Provides mutable access to the canvas context.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [`CanvasContext`].
    pub fn as_canvas(&mut self) -> &mut CanvasContext {
        self.as_mut()
    }
}

impl AsMut<CanvasContext> for Context {
    fn as_mut(&mut self) -> &mut CanvasContext {self.base_context.as_mut()}
}

impl AsMut<wgpu_canvas::FontAtlas> for Context {
    fn as_mut(&mut self) -> &mut wgpu_canvas::FontAtlas {self.base_context.as_mut().as_mut()}
}

/// A trait for defining application plugins that can register themselves with a [`Context`].
///
/// Plugins can optionally provide background tasks and must implement an async constructor.
/// Each plugin is stored in a [`HashMap`] keyed by [`TypeId`], enabling dynamic access.
pub trait Plugin {
    /// Optionally defines background tasks that run independently of the main application context.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - A mutable reference to a [`HeadlessContext`].
    ///
    /// # Returns
    ///
    /// An asynchronous future that resolves to a list of [`Tasks`]. The default implementation returns an empty list.
    fn background_tasks(_ctx: &mut HeadlessContext) -> impl Future<Output = Tasks> {
        async { vec![] }
    }

    /// Asynchronously creates a new instance of the plugin, possibly registering it with the system.
    ///
    /// # Arguments
    ///
    /// * `ctx` - A mutable reference to the full rendering [`Context`].
    /// * `h_ctx` - A mutable reference to the [`HeadlessContext`].
    ///
    /// # Returns
    ///
    /// A future resolving to a tuple: the initialized plugin instance and its associated [`Tasks`].
    fn new(
        ctx: &mut Context,
        h_ctx: &mut HeadlessContext,
    ) -> impl Future<Output = (Self, Tasks)>
    where
        Self: Sized;
}

/// A dynamic plugin store, mapping each plugin type's [`TypeId`] to a boxed instance.
///
/// This allows type-safe downcasting when retrieving specific plugins from the registry.
pub type Plugins = HashMap<TypeId, Box<dyn std::any::Any>>;
/// The `App` trait defines the structure and lifecycle hooks for an application using the framework.
///
/// An implementing type defines how background tasks, plugins, and the root UI component are initialized.
pub trait App {
    /// Optionally defines background tasks to run before rendering or plugin setup.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - A mutable reference to a [`HeadlessContext`].
    ///
    /// # Returns
    ///
    /// An async future resolving to a list of [`Tasks`]. Defaults to an empty list.
    fn background_tasks(_ctx: &mut HeadlessContext) -> impl Future<Output = Tasks> {
        async { vec![] }
    }

    /// Optionally defines the application's plugins and their background tasks.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - A mutable reference to the rendering [`Context`].
    /// * `_h_ctx` - A mutable reference to a [`HeadlessContext`].
    ///
    /// # Returns
    ///
    /// An async future that resolves to a tuple:
    /// - `Plugins`: A registry of plugin instances.
    /// - `Tasks`: A list of background tasks associated with those plugins.
    ///
    /// Defaults to no plugins and no tasks.
    fn plugins(
        _ctx: &mut Context,
        _h_ctx: &mut HeadlessContext,
    ) -> impl Future<Output = (Plugins, Tasks)> {
        async { (HashMap::new(), vec![]) }
    }

    /// Asynchronously constructs the root [`Drawable`] component of the application.
    ///
    /// # Arguments
    ///
    /// * `ctx` - A mutable reference to the application `Context`.
    ///
    /// # Returns
    ///
    /// An async future resolving to a boxed [`Drawable`], which serves as the root component in the UI tree.
    fn new(ctx: &mut Context) -> impl Future<Output = Box<dyn Drawable>>;
}

/// [`ComponentApp<A>`] is a generic application wrapper that integrates an [`App`] implementation
/// into the framework's [`BaseAppTrait`] system for event-driven UI updates and drawing.
///
/// It manages plugin initialization, event handling, UI construction, and re-rendering.
pub struct ComponentApp<A: App> {
    /// The main application context, containing events, plugins, and base rendering context.
    ctx: Context,

    /// The root drawable component representing the UI.
    app: Box<dyn Drawable>,

    /// The size of the screen in logical pixels.
    screen: (f32, f32),

    /// A built and layout-sized version of the root UI component.
    sized_app: SizedBranch,

    /// Marker for the generic [`App`] type. Not used directly.
    _p: std::marker::PhantomData<A>,

    /// Time at which the last frame started rendering. Used for frame timing.
    time: Instant,
}

impl<A: App> BaseAppTrait<Canvas> for ComponentApp<A> {
    /// Controls the default logging level for this app.
    const LOG_LEVEL: log::Level = log::Level::Error;

    /// Runs any headless background tasks defined by the [`App`].
    async fn background_tasks(ctx: &mut HeadlessContext) -> Tasks {
        A::background_tasks(ctx).await
    }

    /// Initializes the app:
    /// - Constructs the [`Context`].
    /// - Loads plugins and their tasks.
    /// - Builds the root UI component.
    /// - Calculates the initial layout.
    async fn new(
        base_ctx: base::Context<Canvas>,
        h_ctx: &mut HeadlessContext,
        width: f32,
        height: f32,
    ) -> (Self, Tasks) {
        let mut ctx = Context::new(base_ctx);
        let (plugins, tasks) = A::plugins(&mut ctx, h_ctx).await;
        ctx.plugins = plugins;

        let mut app = A::new(&mut ctx).await;
        let size_request = _Drawable::request_size(&*app, &mut ctx);
        let screen = (width, height);
        let sized_app = app.build(&mut ctx, screen, size_request);

        (
            ComponentApp {
                ctx,
                app,
                screen,
                sized_app,
                _p: std::marker::PhantomData::<A>,
                time: Instant::now(),
            },
            tasks,
        )
    }

    /// Handles canvas-level events including window resizing, mouse/keyboard input,
    /// and the frame-tick signal which triggers rendering and event processing.
    fn on_event(&mut self, event: canvas::Event) {
        match event {
            canvas::Event::Resized { width, height }
            | canvas::Event::Resumed { width, height } => {
                self.screen = (width, height);
            }
            canvas::Event::Mouse { position, state } => {
                self.ctx
                    .events
                    .push_back(Box::new(MouseEvent { position: Some(position), state }));
            }
            canvas::Event::Keyboard { key, state } => {
                self.ctx
                    .events
                    .push_back(Box::new(KeyboardEvent { key, state }));
            }
            canvas::Event::Tick => {
                log::error!("last_frame: {:?}", self.time.elapsed());
                self.time = Instant::now();

                self.app.event(&mut self.ctx, self.sized_app.clone(), Box::new(TickEvent));

                while let Some(event) = self.ctx.events.pop_front() {
                    if let Some(event) = event
                        .pass(&mut self.ctx, vec![((0.0, 0.0), self.sized_app.0)])
                        .remove(0)
                    {
                        self.app.event(&mut self.ctx, self.sized_app.clone(), event);
                    }
                }

                let size_request = _Drawable::request_size(&*self.app, &mut self.ctx);
                self.sized_app =
                    self.app.build(&mut self.ctx, self.screen, size_request);

                self.app.draw(
                    &mut self.ctx,
                    self.sized_app.clone(),
                    (0.0, 0.0),
                    (0.0, 0.0, self.screen.0, self.screen.1),
                );
            }
            _ => {}
        }
    }

    /// Called during shutdown to return the internal base rendering context.
    async fn close(self) -> base::Context<Canvas> {
        self.ctx.base_context
    }

    /// Returns a mutable reference to the internal base rendering context.
    fn ctx(&mut self) -> &mut base::Context<Canvas> {
        &mut self.ctx.base_context
    }
}


#[macro_export]
macro_rules! create_entry_points {
    ($app:ty) => {
        create_base_entry_points!(Canvas, ComponentApp::<$app>);
    };
}

use crate::base;
use crate::base::driver::state::State;
use crate::base::driver::runtime::Tasks;
use crate::base::driver::camera::Camera;
use crate::base::BaseAppTrait;

use crate::base::renderer::wgpu_canvas as canvas;

use std::future::Future;
use std::time::Instant;

pub use canvas::{Canvas, CanvasItem, Area, Image, Text, Font, Shape, Color, Event, Span, Align};
pub use canvas::{MouseState, KeyboardState, NamedKey, SmolStr, Key, Cursor};
pub use crate::base::HeadlessContext;

/// The core trait for building applications in the UI framework.
///
/// [`App`] defines the lifecycle of an application, including:
/// - optional background tasks to run headlessly
/// - initialization logic
/// - runtime event handling
pub trait App {
    /// Runs any background (headless) tasks associated with this app.
    ///
    /// This is typically used to spawn tasks like networking, data loading,
    /// or async services unrelated to UI drawing.
    fn background_tasks(ctx: &mut HeadlessContext) -> impl Future<Output = Tasks>;

    /// Initializes the app and returns the instance along with any async tasks to be run.
    ///
    /// Called once when the app is first launched. The [`Context`] gives access
    /// to plugins, rendering APIs, and other state.
    fn new(ctx: &mut Context<'_>) -> impl Future<Output = (Self, Tasks)>
    where
        Self: Sized;

    /// Called on each incoming event, including user input or system triggers.
    ///
    /// This is where internal app state should be updated in response to interaction.
    fn on_event(&mut self, ctx: &mut Context<'_>, event: Event);
}

/// A short-lived context object used during app initialization and event handling.
///
/// [`Context`] provides access to screen size and the underlying mutable canvas context,
/// along with plugin state and other systems via the embedded `base::Context`.
pub struct Context<'a> {
    /// The size of the current screen or renderable area.
    pub size: (f32, f32),

    /// A mutable reference to the internal base context, which includes access
    /// to the `canvas::Context` and other rendering systems.
    pub base_context: &'a mut base::Context<'a, Canvas>,
}

impl AsMut<canvas::Context> for Context<'_> {
    /// Allows [`Context`] to be used wherever a mutable reference to the
    /// [`canvas::Context`] is required.
    fn as_mut(&mut self) -> &mut canvas::Context {
        self.base_context.as_mut()
    }
}

impl<'a> Context<'a> {
    /// Creates a new UI context for the current frame or interaction phase.
    ///
    /// # Arguments
    /// - `size`: The size of the screen or rendering area.
    /// - `base_context`: A mutable reference to the underlying base context
    ///   which manages plugins, canvas state, and system access.
    ///
    /// # Returns
    /// A [`Context`] that provides scoped access to rendering and state management.
    fn new(
        size: (f32, f32),
        base_context: &'a mut base::Context<'a, Canvas>
    ) -> Self {
        Context { size, base_context }
    }

    /// Clears the entire canvas with the given color.
    ///
    /// Typically called at the beginning of a draw cycle.
    pub fn clear(&mut self, color: Color) {
        self.base_context.as_mut().clear(color);
    }

    /// Draws a single item to a specified area on the canvas.
    ///
    /// # Arguments
    /// - `area`: The rectangular region to draw into.
    /// - `item`: The drawable content (text, image, shape, etc.)
    pub fn draw(&mut self, area: Area, item: CanvasItem) {
        self.base_context.as_mut().draw(area, item);
    }

    /// Registers a new font from raw binary data.
    ///
    /// # Arguments
    /// - `font`: A byte slice representing a TTF or OTF font file.
    ///
    /// # Returns
    /// A handle to the font for later use in drawing text.
    pub fn add_font(&mut self, font: &[u8]) -> Font {
        self.base_context.as_mut().add_font(font)
    }

    /// Registers an RGBA image for use on the canvas.
    ///
    /// # Arguments
    /// - `image`: An RGBA image from the `image` crate.
    ///
    /// # Returns
    /// A handle to the image for later use in drawing.
    pub fn add_image(&mut self, image: image::RgbaImage) -> Image {
        self.base_context.as_mut().add_image(image)
    }

    /// Returns the current screen size.
    ///
    /// Useful for layout calculations or conditional UI logic.
    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    /// Returns a mutable reference to the global application state.
    ///
    /// Allows reading or modifying shared values during the app lifecycle.
    pub fn state(&mut self) -> &mut State {
        self.base_context.state()
    }

    /// Opens and returns a new camera instance.
    ///
    /// Use this to access video or image capture features.
    pub fn open_camera() -> Camera {
        Camera::new()
    }
}

/// A wrapper around an [`App`] that runs on a canvas-based rendering backend.
/// 
/// Responsible for managing app lifecycle, handling events, and timing frame execution.
pub struct CanvasApp<A: App> {
    /// Current screen dimensions (width, height).
    size: (f32, f32),
    /// The application logic instance implementing [`App`].
    app: A,
    /// Timestamp of the last frame, used for timing metrics.
    time: Instant,
}

impl<A: App> BaseAppTrait<Canvas> for CanvasApp<A> {
    /// Logging level for this application; can be adjusted as needed.
    const LOG_LEVEL: log::Level = log::Level::Error;

    /// Launches background tasks for the app if any.
    ///
    /// Called once at initialization.
    async fn background_tasks(ctx: &mut HeadlessContext) -> Tasks {
        A::background_tasks(ctx).await
    }

    /// Initializes the canvas app.
    ///
    /// # Arguments
    /// - `base_context`: The shared base context for canvas operations.
    /// - `_ctx`: Headless context for setting up global state or plugins.
    /// - `width`, `height`: Initial screen dimensions.
    ///
    /// # Returns
    /// A tuple of the [`CanvasApp`] instance and any background tasks it needs to run.
    async fn new<'a>(
        base_context: &'a mut base::Context<'a, Canvas>,
        _ctx: &mut HeadlessContext,
        width: f32,
        height: f32
    ) -> (Self, Tasks) {
        let size = (width, height);
        let mut ctx = Context::new(size, base_context);
        let (app, tasks) = A::new(&mut ctx).await;

        (
            CanvasApp {
                size,
                app,
                time: Instant::now(),
            },
            tasks,
        )
    }

    /// Handles a single event during the app's lifecycle.
    ///
    /// Reconstructs a scoped [`Context`] and delegates the event to the app.
    fn on_event<'a>(
        &'a mut self,
        base_context: &'a mut base::Context<'a, Canvas>,
        event: Event
    ) {
        match &event {
            Event::Tick => {
                log::error!("last_frame: {:?}", self.time.elapsed());
                self.time = Instant::now();
            },
            Event::Resumed { width, height } | Event::Resized { width, height } => {
                self.size = (*width, *height);
            },
            _ => {}
        };

        let mut ctx = Context::new(self.size, base_context);
        self.app.on_event(&mut ctx, event);
    }

    /// Called when the application is closing. You can add cleanup here if needed.
    async fn close(self) {
        // No-op for now.
    }
}

#[macro_export]
macro_rules! create_entry_points {
    ($app:ty) => {
        create_base_entry_points!(Canvas, CanvasApp<$app>);
    };
}

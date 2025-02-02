mod winit;
pub use winit::{WinitWindow, WinitApp, Winit};

#[cfg(target_os = "android")]
pub use winit::AndroidApp;

mod wgpu;
pub use wgpu::WgpuCanvasRenderer;

pub type LogLevel = log::Level;

pub use wgpu_canvas::{Shape, Mesh};

pub trait App {
    const LOG_LEVEL: LogLevel = LogLevel::Info;

    fn new() -> impl std::future::Future<Output = Self> where Self: Sized;
    fn draw(&mut self) -> impl std::future::Future<Output = Vec<Mesh>>;
}

pub struct CanvasApp<A: App> {
    window_renderer: WgpuCanvasRenderer,
    app: A
}

impl<A: App> WinitApp for CanvasApp<A> {
    const LOG_LEVEL: LogLevel = A::LOG_LEVEL;

    async fn new(window: WinitWindow) -> Self {
        CanvasApp{
            window_renderer: WgpuCanvasRenderer::new(window).await,
            app: A::new().await
        }
    }

    async fn prepare(&mut self, width: u32, height: u32, scale_factor: f32) {
        self.window_renderer.prepare(width, height, scale_factor, self.app.draw().await);
    }

    async fn render(&mut self) {
        self.window_renderer.render();
    }
}

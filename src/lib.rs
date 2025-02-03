mod winit;
pub use winit::{WinitWindow, WinitApp, Winit};

#[cfg(target_os = "android")]
pub use winit::AndroidApp;

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;

mod wgpu;
pub use wgpu::WgpuCanvasRenderer;

pub type LogLevel = log::Level;

pub use wgpu_canvas::{Shape, Mesh};

pub trait App {
    const LOG_LEVEL: LogLevel = LogLevel::Info;

    fn new() -> impl std::future::Future<Output = Self> where Self: Sized;
    fn draw(&mut self, width: u32, height: u32) -> impl std::future::Future<Output = Vec<Mesh>>;
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
        println!("wwidth: {}", width);
        let logical_width = width as f32 / scale_factor;
        let logical_height = height as f32 / scale_factor;
        //Logical size should be whole numbers but leaving them as f32 works better for renderers
        self.window_renderer.prepare(width, height, logical_width, logical_height, self.app.draw(logical_width as u32, logical_height as u32).await);
    }

    async fn render(&mut self) {
        self.window_renderer.render();
    }
}

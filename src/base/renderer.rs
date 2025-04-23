use super::WindowHandle;

use std::future::Future;
use std::sync::Arc;

///The Renderer trait is built on top of a Window Initializer
///
///A Renderers Input should be in logical Pixels
///
///The Renderer will return the actual size of the Renderer Target which may differ from the window
///size. The width and height return by the Renderer will fill the screen if rectangle is provided
///of equal size. The width and height are also logically sized
pub trait Renderer {
    type Input;
    type Context;

    fn get_scale<'a>(&'a self, ctx: &'a Self::Context) -> &'a Scale;

    fn new<W: WindowHandle + 'static>(
        window: W, width: u32, height: u32, scale_factor: f64
    ) -> impl Future<Output = (Self, Self::Context, (f32, f32))> where Self: Sized;
        
    fn resize<W: WindowHandle + 'static>(
        &mut self, ctx: &mut Self::Context, new_window: Option<Arc<W>>, width: u32, height: u32, scale_factor: f64
    ) -> impl Future<Output = (f32, f32)>;

    fn draw(&mut self, ctx: &mut Self::Context, input: Self::Input) -> impl Future<Output = ()>;
}

#[derive(Debug, Clone, Copy)]
pub struct Scale(f64);
impl Scale {
    pub fn physical(&self, x: f32) -> f32 {
        (x as f64 * self.0) as f32
    }

    pub fn logical(&self, x: f32) -> f32 {
        (x as f64 / self.0) as f32
    }
}

#[cfg(feature = "wgpu_canvas")]
pub mod wgpu_canvas;

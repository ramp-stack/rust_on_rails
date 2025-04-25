use crate::base::window::{WindowAppTrait, WindowHandle, WindowEvent};

use std::future::Future;
use std::path::PathBuf;

pub trait RenderAppTrait<R: Renderer + ?Sized> {
    fn new(storage_path: PathBuf, ctx: &mut R::Context, width: f32, height: f32) -> impl Future<Output = Self> where Self: Sized;
    fn on_event(&mut self, ctx: &mut R::Context, event: R::Event) -> impl Future<Output = ()>;
    fn close(self, ctx: &mut R::Context) -> impl Future<Output = ()>;
}

pub trait HasLifeEvents {
    fn is_resumed(&self) -> bool;
    fn is_paused(&self) -> bool;
}

pub trait Renderer {
    type Context;
    type Event: HasLifeEvents;

    fn new<W: WindowHandle + 'static>(
        window: W, width: u32, height: u32, scale_factor: f64
    ) -> impl Future<Output = (Self, Self::Context, (f32, f32))> where Self: Sized;
        
    fn on_event<W: WindowHandle, A: RenderAppTrait<Self>>(
        &mut self, ctx: &mut Self::Context, app: &mut A, event: WindowEvent<W>
    ) -> impl Future<Output = ()>;
    fn close(self, ctx: Self::Context) -> impl Future<Output = ()>;
}

pub struct RenderApp<R: Renderer, A: RenderAppTrait<R>>(R, A, R::Context);
impl<A: RenderAppTrait<R>, R: Renderer> WindowAppTrait for RenderApp<R, A> {
    async fn new<W: WindowHandle>(
        storage_path: PathBuf, window: W, width: u32, height: u32, scale_factor: f64
    ) -> Self where Self: Sized {
        let (renderer, mut ctx, size) = R::new(window, width, height, scale_factor).await;
        let app = A::new(storage_path, &mut ctx, size.0, size.1).await;
        RenderApp(renderer, app, ctx)
    }
    async fn on_event<W: WindowHandle>(&mut self, event: WindowEvent<W>) {
        self.0.on_event(&mut self.2, &mut self.1, event).await;
    }
    async fn close(mut self) {self.1.close(&mut self.2).await; self.0.close(self.2).await;}
}


#[cfg(feature = "wgpu_canvas")]
pub mod wgpu_canvas;

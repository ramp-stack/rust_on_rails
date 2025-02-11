use wgpu::{
    SurfaceConfiguration,
    TextureView,
    Surface,
    Device,
    Queue,
    RenderPassDepthStencilAttachment,
    DepthBiasState,
    StencilState,
    CompareFunction,
    DepthStencilState,
    TextureFormat,
    CommandEncoderDescriptor,
    TextureViewDescriptor,
    TextureDescriptor,
    MultisampleState,
    TextureUsages,
    RenderPassColorAttachment,
    RenderPassDescriptor,
    RequestAdapterOptions,
    InstanceDescriptor,
    TextureDimension,
    Instance,
    PowerPreference,
    DeviceDescriptor,
    Operations,
    Features,
    Extent3d,
    StoreOp,
    Limits,
    LoadOp,
    Color
};

use wgpu_canvas::CanvasRenderer;

use std::cmp::min;
use std::time::Instant;

use super::{WinitAppTrait, winit::WinitWindow};

pub use wgpu_canvas::{ItemType, Shape, Text, image, ImageKey, FontKey, DrawCommand};
use wgpu_canvas::{CanvasAtlas, CanvasItem as InnerCanvasItem};

const SAMPLE_COUNT: u32 = 4;


#[derive(Clone, Debug)]
pub struct CanvasItem(pub ItemType, pub (u32, u32), pub Option<(u32, u32, u32, u32)>);

#[derive(Default, Debug)]
struct Size {
    width: u32,
    height: u32,
    scale_factor: f64,
    l_width: u32,
    l_height: u32
}

impl Size {
    pub fn new(device: &Device, mut width: u32, mut height: u32, scale_factor: f64) -> Self {
        let limits = device.limits();
        width = min(width, limits.max_texture_dimension_2d);
        height = min(height, limits.max_texture_dimension_2d);

        Size{
            width, height, scale_factor,
            l_width: (width as f64 / scale_factor) as u32,
            l_height: (height as f64 / scale_factor) as u32
        }
    }

    pub fn to_logical(&self, mut width: u32, mut height: u32) -> (u32, u32) {
        width = min(width, self.width);
        height = min(height, self.height);
        (
            (width as f64 / self.scale_factor).ceil() as u32,
            (height as f64 / self.scale_factor).ceil() as u32
        )
    }
}

pub struct CanvasContext{
    pub atlas: CanvasAtlas,
    components: Option<Vec<InnerCanvasItem>>,
    pub screen_width: u32,
    pub screen_height: u32,
    pub position: (u32, u32)
}

impl Default for CanvasContext {
    fn default() -> Self {
        CanvasContext{
            atlas: CanvasAtlas::default(),
            components: Some(Vec::new()),
            screen_width: 0,
            screen_height: 0,
            position: (0, 0)
        }
    }
}

impl CanvasContext {
    pub fn clear(&mut self, color: &'static str) {
        *self.components.as_mut().unwrap() = vec![InnerCanvasItem(
            ItemType::Shape(Shape::Rectangle(self.screen_width, self.screen_height), color, None),
            u16::MAX, (0, 0), (0, 0, self.screen_width, self.screen_height)
        )];
    }

    pub fn draw(&mut self, item: CanvasItem) {
        let z = u16::MAX-1-(self.components.as_ref().unwrap().len()) as u16;
        let bound = item.2.unwrap_or((0, 0, self.screen_width, self.screen_height));
        self.components.as_mut().unwrap().push(InnerCanvasItem(item.0, z, item.1, bound));
    }
}

pub trait CanvasAppTrait {
    fn new(ctx: &mut CanvasContext) -> impl std::future::Future<Output = Self> where Self: Sized;
    fn draw(&mut self, ctx: &mut CanvasContext) -> impl std::future::Future<Output = ()>;

    fn on_click(&mut self, ctx: &mut CanvasContext) -> impl std::future::Future<Output = ()>;
    fn on_move(&mut self, ctx: &mut CanvasContext) -> impl std::future::Future<Output = ()>;
    fn on_press(&mut self, ctx: &mut CanvasContext, t: String) -> impl std::future::Future<Output = ()>;
}

pub struct CanvasApp<A: CanvasAppTrait> {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: Size,
    msaa_view: Option<TextureView>,
    depth_view: TextureView,
    canvas_renderer: CanvasRenderer,
    context: CanvasContext,

    app: A,

    time: Instant
}

impl<A: CanvasAppTrait> WinitAppTrait for CanvasApp<A> {
    async fn new(window: WinitWindow) -> Self {
        let instance = Instance::new(&InstanceDescriptor::default());

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let mut limits = Limits::downlevel_webgl2_defaults();
        limits.max_texture_dimension_2d = 4096;

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                required_features: Features::empty(),
                required_limits: limits,
                label: None,
                memory_hints: Default::default(),
            },
            None,
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            width: 1,
            height: 1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_caps.formats[0]],
            desired_maximum_frame_latency: 2,
        };

        let multisample = MultisampleState {
            count: SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let depth_stencil = DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        };

        let depth_view = Self::create_depth_view(&device, &config);

        let canvas_renderer = CanvasRenderer::new(&queue, &device, &surface_caps.formats[0], multisample, Some(depth_stencil));

        let mut context = CanvasContext::default();
        let app = A::new(&mut context).await;

        CanvasApp{
            surface,
            device,
            queue,
            config,
            size: Size::default(),
            msaa_view: None,
            depth_view,
            canvas_renderer,
            context,
            app,
            time: Instant::now()
        }

    }

    async fn prepare(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.resize(width, height, scale_factor);

        self.context.screen_width = self.size.l_width;
        self.context.screen_height = self.size.l_height;
        self.app.draw(&mut self.context).await;
        let items = self.context.components.replace(Vec::new()).unwrap();

        self.canvas_renderer.prepare(
            &self.device,
            &self.queue,
            self.size.width,
            self.size.height,
            self.size.scale_factor,
            self.size.l_width as f32,
            self.size.l_height as f32,
            &mut self.context.atlas,
            items
        );
    }

    async fn render(&mut self) {
        //log::error!("last_frame: {}", self.time.elapsed().as_nanos());
        self.time = Instant::now();
        let output = self.surface.get_current_texture().unwrap();
        let frame_view = output.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: if SAMPLE_COUNT > 1 {self.msaa_view.as_ref().unwrap()} else {&frame_view},
                resolve_target: if SAMPLE_COUNT > 1 {Some(&frame_view)} else {None},
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.canvas_renderer.render(&mut rpass);

        drop(rpass);

        self.queue.submit(Some(encoder.finish()));
        output.present();
    }

    async fn on_click(&mut self) {
        self.app.on_click(&mut self.context).await
    }
    async fn on_move(&mut self, x: u32, y: u32) {
        self.context.position = self.size.to_logical(x, y);
        self.app.on_move(&mut self.context).await
    }
    async fn on_press(&mut self, t: String) {
        self.app.on_press(&mut self.context, t).await
    }
}

impl<A: CanvasAppTrait> CanvasApp<A> {
    fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        if
            (width > 0 && height > 0) &&
            (self.config.width != width || self.config.height != height)
        {
            let size = Size::new(&self.device, width, height, scale_factor);
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            if SAMPLE_COUNT > 1 {
                self.msaa_view = Some(Self::create_msaa_view(&self.device, &self.config));
            }
            self.depth_view = Self::create_depth_view(&self.device, &self.config);
            self.size = size;
        }
    }

    fn create_msaa_view(device: &Device, config: &SurfaceConfiguration) -> TextureView {
        device.create_texture(&TextureDescriptor{
            label: Some("Multisampled frame descriptor"),
            size: Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: TextureDimension::D2,
            format: config.format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor::default())
    }

    fn create_depth_view(device: &Device, config: &SurfaceConfiguration) -> TextureView {
        device.create_texture(&TextureDescriptor {
            label: Some("Depth Stencil Texture"),
            size: Extent3d { // 2.
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor::default())
    }
}

#[macro_export]
macro_rules! create_canvas_entry_points {
    ($app:ty) => {
        create_winit_entry_points!(CanvasApp::<$app>);
    };
}

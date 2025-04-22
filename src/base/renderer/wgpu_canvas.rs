use wgpu::{RenderPassDepthStencilAttachment, RenderPassColorAttachment, CommandEncoderDescriptor, TextureViewDescriptor, RequestAdapterOptions, SurfaceConfiguration, RenderPassDescriptor, InstanceDescriptor, DepthStencilState, TextureDescriptor, TextureDimension, MultisampleState, DeviceDescriptor, PowerPreference, CompareFunction, WindowHandle, DepthBiasState, TextureUsages, TextureFormat, StencilState, TextureView, Operations, Instance, Features, Extent3d, Surface, StoreOp, LoadOp, Limits, Device, Queue};

use wgpu_canvas::{CanvasRenderer, ImageAtlas, FontAtlas};

use std::sync::Arc;

use super::{Renderer, HasScale, Scale};

pub use wgpu_canvas::{Shape, Color, Image, Font, Area};

const SAMPLE_COUNT: u32 = 4;

pub struct CanvasContext {
    scale: Scale,
    image: ImageAtlas,
    font: FontAtlas
}

impl CanvasContext {
    pub fn add_font(&mut self, font: &[u8]) -> Font {self.font.add(font)}
    pub fn add_image(&mut self, image: image::RgbaImage) -> Image {self.image.add(image)}
}

impl HasScale for CanvasContext {
    fn get_scale(&self) -> &Scale {&self.scale}
}

impl AsMut<FontAtlas> for CanvasContext {
    fn as_mut(&mut self) -> &mut FontAtlas {&mut self.font}
}

#[derive(Clone, Debug)]
pub struct Text(wgpu_canvas::Text);

impl Text {
    pub fn new(ctx: &mut impl AsMut<CanvasContext>,
        text: &str, color: Color, font: Font,
        size: f32, line_height: f32, width: Option<f32>
    ) -> Self {
        let scale = *ctx.as_mut().get_scale();
        Text(wgpu_canvas::Text::new(
            ctx.as_mut().as_mut(), text, color, font,
            scale.physical(size),
            scale.physical(line_height),
            width.map(|w| scale.physical(w)),
        ))
    }

    pub fn text(&mut self) -> &mut String {&mut self.0.text}

    pub fn color(&mut self) -> &mut Color {&mut self.0.color}

    pub fn size(&self, ctx: &mut impl AsMut<CanvasContext>) -> (f32, f32) {
        let size = self.0.size();
        (ctx.as_mut().scale.logical(size.0),
        ctx.as_mut().scale.logical(size.1))
    }
}

#[derive(Clone, Debug)]
pub enum CanvasItem {
    Shape(Shape, Color),
    Image(Shape, Image, Option<Color>),
    Text(Text),
}

impl CanvasItem {
    fn scale(self, scale: &Scale) -> wgpu_canvas::CanvasItem {
        match self {
            CanvasItem::Shape(shape, color) => wgpu_canvas::CanvasItem::Shape(
                Self::scale_shape(shape, scale), color
            ),
            CanvasItem::Image(shape, image, color) => wgpu_canvas::CanvasItem::Image(
                Self::scale_shape(shape, scale), image, color
            ),
            CanvasItem::Text(text) => wgpu_canvas::CanvasItem::Text(text.0)
        }
    }

    fn scale_shape(shape: Shape, scale: &Scale) -> Shape {
        match shape {
            Shape::Ellipse(s, size) => Shape::Ellipse(scale.physical(s), Self::scale_size(size, scale)),
            Shape::Rectangle(s, size) => Shape::Rectangle(scale.physical(s), Self::scale_size(size, scale)),
            Shape::RoundedRectangle(s, size, r) => Shape::RoundedRectangle(
                scale.physical(s), Self::scale_size(size, scale), scale.physical(r)
            ),
        }
    }

    fn scale_size(size: (f32, f32), scale: &Scale) -> (f32, f32) {
        (scale.physical(size.0), scale.physical(size.1))
    }
}

pub struct Canvas {
    instance: Instance,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    msaa_view: Option<TextureView>,
    depth_view: TextureView,
    canvas_renderer: CanvasRenderer,
}

impl Canvas {
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

    fn scale_area(area: Area, scale: &Scale) -> Area {
        Area(
            (scale.physical(area.0.0), scale.physical(area.0.1)),
            area.1.map(|(x, y, w, h)| (
                scale.physical(x), scale.physical(y),
                scale.physical(w), scale.physical(h)
            ))
        )
    }
}

impl Renderer for Canvas {
    type Input = Vec<(Area, CanvasItem)>;
    type Context = CanvasContext;

    fn get_scale<'a>(&'a self, ctx: &'a Self::Context) -> &'a Scale {&ctx.scale}

    async fn new<W: WindowHandle + 'static>(
        window: W, width: u32, height: u32, scale_factor: f64
    ) -> (Self, Self::Context, (f32, f32)) {
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
        limits.max_texture_dimension_2d = 8192;

        let width = width.min(limits.max_texture_dimension_2d);
        let height = height.min(limits.max_texture_dimension_2d);

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
            width,
            height,
            format: surface_caps.formats[0],
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_caps.formats[0]],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let multisample = MultisampleState {
            count: SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: true,
        };

        let depth_stencil = DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        };

        let msaa_view = (SAMPLE_COUNT > 1).then(|| Self::create_msaa_view(&device, &config));

        let depth_view = Self::create_depth_view(&device, &config);

        let canvas_renderer = CanvasRenderer::new(&queue, &device, &surface_caps.formats[0], multisample, Some(depth_stencil));

        let scale = Scale(scale_factor);
        let size = (scale.logical(width as f32), scale.logical(height as f32));
        (Canvas{
            instance,
            surface,
            device,
            queue,
            config,
            msaa_view,
            depth_view,
            canvas_renderer,
        }, 
        CanvasContext{
            scale,
            image: ImageAtlas::default(),
            font: FontAtlas::default()
        }, size)
    }

    async fn resize<W: WindowHandle + 'static>(
        &mut self, ctx: &mut Self::Context, new_window: Option<Arc<W>>, width: u32, height: u32, scale_factor: f64
    ) -> (f32, f32) {
        ctx.scale.0 = scale_factor;
        if let Some(new_window) = new_window {
            self.surface = self.instance.create_surface(new_window).unwrap();
        }
        if width > 0 && height > 0 {
            let limits = self.device.limits();
            self.config.width = width.min(limits.max_texture_dimension_2d);
            self.config.height = height.min(limits.max_texture_dimension_2d);
            self.surface.configure(&self.device, &self.config);
            if SAMPLE_COUNT > 1 {
                self.msaa_view = Some(Self::create_msaa_view(&self.device, &self.config));
            }
            self.depth_view = Self::create_depth_view(&self.device, &self.config);
        }

        (ctx.scale.logical(self.config.width as f32), ctx.scale.logical(self.config.height as f32))
    }

    async fn draw(&mut self, ctx: &mut Self::Context, input: Self::Input) {
        let items = input.into_iter().map(|(a, i)|
            (Self::scale_area(a, &ctx.scale), i.scale(&ctx.scale))
        ).collect();

        self.canvas_renderer.prepare(
            &self.device,
            &self.queue,
            self.config.width as f32,
            self.config.height as f32,
            &mut ctx.image,
            &mut ctx.font,
            items
        );

        let output = self.surface.get_current_texture().unwrap();
        let frame_view = output.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: if SAMPLE_COUNT > 1 {self.msaa_view.as_ref().unwrap()} else {&frame_view},
                resolve_target: if SAMPLE_COUNT > 1 {Some(&frame_view)} else {None},
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
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
}

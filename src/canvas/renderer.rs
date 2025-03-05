use wgpu::{RenderPassDepthStencilAttachment, RenderPassColorAttachment, CommandEncoderDescriptor, TextureViewDescriptor, RequestAdapterOptions, SurfaceConfiguration, RenderPassDescriptor, InstanceDescriptor, DepthStencilState, TextureDescriptor, TextureDimension, MultisampleState, DeviceDescriptor, PowerPreference, CompareFunction, DepthBiasState, TextureUsages, TextureFormat, StencilState, TextureView, Operations, Instance, Features, Extent3d, Surface, StoreOp, LoadOp, Limits, Device, Queue};

use crate::winit::WinitWindow;
use wgpu_canvas::{CanvasRenderer, CanvasAtlas, CanvasItem};

use std::cmp::min;

const SAMPLE_COUNT: u32 = 4;

pub struct Canvas {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    msaa_view: Option<TextureView>,
    depth_view: TextureView,
    canvas_renderer: CanvasRenderer,
}

impl Canvas {
    pub async fn new(window: WinitWindow) -> Self {
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
            alpha_to_coverage_enabled: true,
        };

        let depth_stencil = DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        };

        let depth_view = Self::create_depth_view(&device, &config);

        let canvas_renderer = CanvasRenderer::new(&queue, &device, &surface_caps.formats[0], multisample, Some(depth_stencil));

        Canvas{
            surface,
            device,
            queue,
            config,
            msaa_view: None,
            depth_view,
            canvas_renderer,
        }
    }

    pub fn prepare(&mut self, atlas: &mut CanvasAtlas, items: Vec<CanvasItem>) {
        self.canvas_renderer.prepare(
            &self.device,
            &self.queue,
            self.config.width,
            self.config.height,
            atlas, items
        );
    }

    pub fn render(&mut self) {
        let output = self.surface.get_current_texture().unwrap();
        let frame_view = output.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: if SAMPLE_COUNT > 1 {self.msaa_view.as_ref().unwrap()} else {&frame_view},
                resolve_target: if SAMPLE_COUNT > 1 {Some(&frame_view)} else {None},
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color::TRANSPARENT),
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

    pub fn resize(&mut self, width: u32, height: u32) -> (u32, u32) {
        if width > 0 && height > 0 {
            let limits = self.device.limits();
            self.config.width = min(width, limits.max_texture_dimension_2d);
            self.config.height = min(height, limits.max_texture_dimension_2d);
            self.surface.configure(&self.device, &self.config);
            if SAMPLE_COUNT > 1 {
                self.msaa_view = Some(Self::create_msaa_view(&self.device, &self.config));
            }
            self.depth_view = Self::create_depth_view(&self.device, &self.config);
        }

        (self.config.width, self.config.height)
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

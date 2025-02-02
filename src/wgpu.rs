use wgpu::{
    SurfaceConfiguration,
    TextureView,
    Surface,
    Device,
    Queue,
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
    SurfaceTarget,
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

use std::cmp::min;

use wgpu_canvas::{CanvasRenderer, Mesh};

const MAX_SIZE: u32 = 2048;//Not sure why it panics otherwise
const SAMPLE_COUNT: u32 = 4;

pub struct WgpuCanvasRenderer {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    msaa_view: Option<TextureView>,
    canvas: CanvasRenderer
}

impl WgpuCanvasRenderer {
    pub async fn new(window: impl Into<SurfaceTarget<'static>>) -> Self {
        let instance = Instance::new(InstanceDescriptor::default());

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                required_features: Features::empty(),
                required_limits: Limits::downlevel_webgl2_defaults(),
                label: None,
                memory_hints: Default::default(),
            },
            None,
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            width: 0,
            height: 0,
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

        let canvas = CanvasRenderer::new(&device, &surface_caps.formats[0], multisample, None);

        WgpuCanvasRenderer{
            surface,
            device,
            queue,
            config,
            msaa_view: None,
            canvas
        }
    }

    pub fn prepare(&mut self, width: u32, height: u32, _scale_factor: f32, meshes: Vec<Mesh>) {
        self.resize(width, height);
        self.canvas.prepare(&self.device, &self.queue, meshes);
    }


    pub fn render(&mut self) {
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
                    load: LoadOp::Clear(Color::WHITE),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.canvas.render(&mut rpass);

        drop(rpass);

        self.queue.submit(Some(encoder.finish()));
        output.present();
    }


    fn resize(&mut self, width: u32, height: u32) {
        if
            (width > 0 && height > 0) &&
            (self.config.width != width || self.config.height != height)
        {
            self.config.width = min(width, MAX_SIZE);
            self.config.height = min(height, MAX_SIZE);
            self.surface.configure(&self.device, &self.config);
            if SAMPLE_COUNT > 1 {
                self.msaa_view = Some(Self::create_msaa_view(&self.device, &self.config));
            }
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
}

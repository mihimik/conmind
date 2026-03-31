use wgpu::util::DeviceExt;
use winit::window::Window;
use std::sync::Arc;
use wgpu::{BackendOptions, MemoryBudgetThresholds};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct WindowSizeUniform {
    width: f32,
    height: f32,
    _padding: [f32; 2],
}

pub struct RenderContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub staging_belt: wgpu::util::StagingBelt,
    pub scale_factor: f32,
    pub window_size_buffer: wgpu::Buffer,
    pub window_size_bind_group: wgpu::BindGroup,
    pub window_size_bind_group_layout: wgpu::BindGroupLayout,
}

impl<'a> RenderContext {
    pub async fn new(window: Arc<Window>) -> RenderContext {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::VALIDATION,
            memory_budget_thresholds: MemoryBudgetThresholds::default(),
            backend_options: BackendOptions::default(),
            display: None,
        });

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::POLYGON_MODE_LINE,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let alpha_mode = surface_caps.alpha_modes.iter().copied().find(|&mode| {
            mode == wgpu::CompositeAlphaMode::PostMultiplied || mode == wgpu::CompositeAlphaMode::PreMultiplied
        }).unwrap_or(surface_caps.alpha_modes[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        println!("Selected surface format: {:?}", surface_format);
        println!("Selected alpha mode: {:?}", config.alpha_mode);

        let scale_factor = &window.scale_factor().clone();

        let window_size_uniform = WindowSizeUniform {
            width: config.width as f32,
            height: config.height as f32,
            _padding: [0.0; 2],
        };
        let window_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Window Size Buffer"),
            contents: bytemuck::cast_slice(&[window_size_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let window_size_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Window Size BGL"),
        });

        let window_size_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &window_size_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: window_size_buffer.as_entire_binding(),
            }],
            label: Some("Window Size BG"),
        });

        let staging_belt = wgpu::util::StagingBelt::new(device.clone(), 1024);

        Self {
            device: device.clone(),
            queue,
            surface,
            config,
            size,
            staging_belt,
            scale_factor: *scale_factor as f32,
            window_size_buffer,
            window_size_bind_group,
            window_size_bind_group_layout,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;

            self.surface.configure(&self.device, &self.config);

            let window_size_uniform = WindowSizeUniform {
                width: new_size.width as f32,
                height: new_size.height as f32,
                _padding: [0.0; 2],
            };
            self.queue.write_buffer(&self.window_size_buffer, 0, bytemuck::cast_slice(&[window_size_uniform]));
        }
    }
}
use crate::render_context::RenderContext;
use crate::audio::AudioData;

use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;
use winit::window::Window;
use winit::dpi::PhysicalSize;
use crate::audio;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AudioUniform {
    pub time: f32,
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
    pub volume: f32,
    pub _padding: [f32; 3],
}

impl AudioUniform {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            bass: 0.0,
            mid: 0.0,
            high: 0.0,
            volume: 0.0,
            _padding: [0.0; 3],
        }
    }
}

pub struct State {
    pub render_ctx: RenderContext,
    pub window: Arc<Window>,
    pub size: PhysicalSize<u32>,

    pub audio_buffer: wgpu::Buffer,
    pub audio_bind_group: wgpu::BindGroup,
    pub audio_data: Arc<Mutex<AudioData>>,
    pub audio_stream: cpal::Stream,

    pub smooth_audio_data: AudioUniform,
    pub total_time: f32,

    pub max_high: f32,

    pub pipeline: Option<wgpu::RenderPipeline>,
}

impl State {
    pub async fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let render_ctx = RenderContext::new(Arc::clone(&window)).await;

        let (audio_buffer, audio_bind_group_layout, audio_bind_group) = init_audio(render_ctx.device.clone(), render_ctx.window_size_buffer.clone());

        let render_pipeline_layout = render_ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Main Layout"),
            bind_group_layouts: &[Some(&audio_bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = create_render_pipeline(
            Some("Main Pipeline"),
            &render_ctx.device,
            &render_pipeline_layout,
            render_ctx.config.format,
            None,
            &[],
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::include_wgsl!("presets/glitch_barocco.wgsl"),
            wgpu::PolygonMode::Fill,
        );

        let size = window.inner_size();

        let (audio_data, audio_stream) = audio::setup_audio();

        Self {
            render_ctx,
            window,
            size,
            audio_buffer,
            audio_bind_group,
            audio_data,
            audio_stream,
            smooth_audio_data: AudioUniform::new(),
            max_high: 0.01,
            pipeline,
            total_time: 0.0,
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::CreateSurfaceError> {
        let output = self.render_ctx.surface.get_current_texture();

        match output {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => {
                let texture = surface_texture.texture.clone();
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = self.render_ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                        multiview_mask: None,
                    });

                    render_pass.set_viewport(0.0, 0.0, self.size.width as f32, self.size.height as f32, 0.0, 1.0);

                    if let Some(ref p) = self.pipeline {
                        render_pass.set_pipeline(p);
                        render_pass.set_bind_group(0, &self.audio_bind_group, &[]);
                        render_pass.draw(0..3, 0..1);
                    }
                }

                self.render_ctx.queue.submit(std::iter::once(encoder.finish()));

                surface_texture.present();
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.resize(self.size);
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                eprintln!("Surface timeout.");
            }
            _ => {}
        }

        Ok(())
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        let delta = dt.as_secs_f32();
        let audio_shared = self.audio_data.lock().unwrap();

        let smoothness: f32 = 0.15;

        self.smooth_audio_data.bass += (audio_shared.bass - self.smooth_audio_data.bass) * smoothness.min(1.0);
        self.smooth_audio_data.mid += (audio_shared.mid - self.smooth_audio_data.mid) * smoothness.min(1.0);

        if audio_shared.high > self.max_high {
            self.max_high = audio_shared.high;
        } else {
            self.max_high -= 0.1 * delta;
        }
        self.max_high = self.max_high.max(0.01);

        let normalized_high = (audio_shared.high / self.max_high).min(1.0);
        let laser_signal = normalized_high.powf(2.0);

        self.smooth_audio_data.high = laser_signal;
        // println!("{}", laser_signal);

        let speed = 0.5 + (audio_shared.bass * 2.0);
        self.total_time += delta * speed;

        let mut data = self.smooth_audio_data;
        data.time = self.total_time;

        let aggression = (audio_shared.high + audio_shared.mid ) * audio_shared.bass;
        data.volume = aggression;

        self.render_ctx.queue.write_buffer(&self.audio_buffer, 0, bytemuck::cast_slice(&[data]));
    }


    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.render_ctx.resize(new_size);
        self.size = new_size;
    }
}

fn init_audio(device: wgpu::Device, window_size_buffer: wgpu::Buffer) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
    let audio_uniform = AudioUniform::new();

    let spectrum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Spectrum Buffer"),
        size: (512 * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let audio_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Audio Buffer"),
        contents: bytemuck::cast_slice(&[audio_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let audio_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Audio BGL"),
        });
    let audio_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &audio_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: audio_buffer.as_entire_binding(),
        }, wgpu::BindGroupEntry {
            binding: 1,
            resource: spectrum_buffer.as_entire_binding(),
        }, wgpu::BindGroupEntry {
            binding: 2,
            resource: window_size_buffer.as_entire_binding(),
        }],
        label: Some("Audio BG"),
    });

    (audio_buffer, audio_bind_group_layout, audio_bind_group)
}

fn create_render_pipeline(
    label: Option<&str>,
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    topology: wgpu::PrimitiveTopology,
    shader: wgpu::ShaderModuleDescriptor,
    polygon_mode: wgpu::PolygonMode,
) -> Option<wgpu::RenderPipeline> {
    let error_guard = device.push_error_scope(wgpu::ErrorFilter::Validation);

    let shader = device.create_shader_module(shader);
    let error = pollster::block_on(error_guard.pop());

    if let Some(e) = error {
        println!("Shader error: {}", e);
        None
    } else {
        println!("Shader successfully created.");
        Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label.unwrap_or("Render Pipeline")),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: vertex_layouts,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        }))
    }
}
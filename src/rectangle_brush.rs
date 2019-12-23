const DEFAULT_MAX_RECTS: usize = 100;

#[derive(Debug, Clone, Copy)]
struct RectInstance {
    left_top: [f32; 2],
    right_bottom: [f32; 2],
    color: [f32; 4],
}

pub struct RectangleBrush {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    transform_buffer: wgpu::Buffer,
    instance_queue: Vec<RectInstance>,
    rect_capacity: usize,
    current_transform: [f32; 16],
}

#[cfg_attr(rustfmt, rustfmt_skip)]
const IDENTITY_MATRIX: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

fn orthographic_projection(width: f64, height: f64) -> [f32; 16] {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    [
        2.0 / width as f32, 0.0, 0.0, 0.0,
        0.0, 2.0 / height as f32, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, -1.0, 0.0, 1.0,
    ]
}

impl RectangleBrush {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> RectangleBrush {
        let vs_bytes = include_bytes!("shaders/rectangle.vert.spv");
        let vs_module = device
            .create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs_bytes[..])).unwrap());

        let fs_bytes = include_bytes!("shaders/rectangle.frag.spv");
        let fs_module = device
            .create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs_bytes[..])).unwrap());

        let transform_buffer = device
            .create_buffer_mapped(16, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
            .fill_from_slice(&IDENTITY_MATRIX);

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: (std::mem::size_of::<RectInstance>() * DEFAULT_MAX_RECTS) as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[wgpu::BindGroupLayoutBinding {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &transform_buffer,
                    range: 0..(16 * 4),
                },
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Cw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
            color_states: &[wgpu::ColorStateDescriptor {
                format: format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<RectInstance>() as u64,
                step_mode: wgpu::InputStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float2,
                        offset: 4 * 2,
                    },
                    wgpu::VertexAttributeDescriptor {
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float4,
                        offset: 4 * 4,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        RectangleBrush {
            pipeline,
            bind_group,
            transform_buffer,
            instance_buffer,
            current_transform: IDENTITY_MATRIX,
            instance_queue: vec![],
            rect_capacity: DEFAULT_MAX_RECTS,
        }
    }

    pub fn queue_rectangle(&mut self, x: i32, y: i32, width: i32, height: i32, color: [f32; 4]) {
        let instance = RectInstance {
            left_top: [x as f32, y as f32],
            right_bottom: [(x + width) as f32, (y + height) as f32],
            color,
        };
        self.instance_queue.push(instance);
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: (f64, f64),
    ) {
        if self.instance_queue.is_empty() {
            return;
        }

        let instance_count = self.instance_queue.len();

        // If we have more rectangles than the instance buffer can fit, resize instance buffer.
        if instance_count > self.rect_capacity {
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                size: (std::mem::size_of::<RectInstance>() * instance_count) as u64,
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            });

            self.rect_capacity = instance_count;
        }

        let temp_buffer = device
            .create_buffer_mapped(instance_count, wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(self.instance_queue.as_slice());
        self.instance_queue.clear();

        encoder.copy_buffer_to_buffer(
            &temp_buffer,
            0,
            &self.instance_buffer,
            0,
            (std::mem::size_of::<RectInstance>() * instance_count) as u64,
        );

        let ortho_proj = orthographic_projection(size.0, size.1);
        if self.current_transform != ortho_proj {
            let temp_buffer = device
                .create_buffer_mapped(16, wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&ortho_proj[..]);

            encoder.copy_buffer_to_buffer(&temp_buffer, 0, &self.transform_buffer, 0, 16 * 4);

            self.current_transform = ortho_proj;
        }

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: view,
                resolve_target: None,
                load_op: wgpu::LoadOp::Load,
                store_op: wgpu::StoreOp::Store,
                clear_color: wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                },
            }],
            depth_stencil_attachment: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffers(0, &[(&self.instance_buffer, 0)]);
        rpass.draw(0..4, 0..instance_count as u32);
    }
}

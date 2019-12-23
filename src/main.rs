// TODO
// * Do layout before sending to wgpu_glyph so we know glyph positions
// * Add pipeline for drawing rectangles

use zerocopy::AsBytes as _;

use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder, Scale, Section, SectionText, VariedSection};
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[derive(Debug, Clone, Copy)]
struct RectInstance {
    left_top: [f32; 2],
    right_bottom: [f32; 2],
    color: [f32; 4],
}

struct RectangleBrush {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    transform_buffer: wgpu::Buffer,
    instance_queue: Vec<RectInstance>,
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
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> RectangleBrush {
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
            // TODO: Don't hardcode max of 10 rects
            size: std::mem::size_of::<RectInstance>() as u64 * 10,
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
        }
    }

    fn queue_rectangle(&mut self, x: i32, y: i32, width: i32, height: i32, color: [f32; 4]) {
        let instance = RectInstance {
            left_top: [x as f32, y as f32],
            right_bottom: [(x + width) as f32, (y + height) as f32],
            color,
        };
        self.instance_queue.push(instance);
    }

    fn draw(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: (f64, f64),
    ) {
        if self.instance_queue.is_empty() {
            return;
        }

        assert!(
            self.instance_queue.len() <= 10,
            "Cannot draw more than 10 rects"
        );
        let instance_count = self.instance_queue.len();

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

struct Buffer {
    lines: Vec<String>,
    scroll: f32,
}

impl Buffer {
    fn new() -> Buffer {
        let file = include_str!("main.rs");
        Buffer {
            scroll: 0.0,
            lines: file.lines().map(|line| line.to_owned()).collect(),
        }
    }

    fn scroll(&mut self, delta: f32) {
        let max_scroll = if self.lines.is_empty() {
            0.0
        } else {
            // TODO: Find better way to calculate max scroll based on line count
            ((self.lines.len() - 1) as f32 * 40.0) + 5.0
        };

        self.scroll = (self.scroll + delta).max(0.0).min(max_scroll);
    }

    fn draw(
        &self,
        size: PhysicalSize,
        glyph_brush: &mut GlyphBrush<()>,
        rect_brush: &mut RectangleBrush,
    ) {
        let x = 10.0;
        let digit_count = self.lines.len().to_string().chars().count();
        let gutter_offset = 20.0 + digit_count as f32 * 20.0;
        let mut y = 5.0 - self.scroll;

        let active_line = 40;
        let cursor = 10;

        for (index, line) in self.lines.iter().enumerate() {
            if y < -40.0 {
                y += 40.0;
                continue;
            }
            if y > size.height as f32 {
                break;
            }

            if index == active_line {
                // active line
                rect_brush.queue_rectangle(
                    0,
                    y as i32,
                    size.width as i32,
                    40,
                    [0.05, 0.05, 0.05, 1.0],
                );

                rect_brush.queue_rectangle(40 * cursor, y as i32, 4, 40, [1.0, 1.0, 1.0, 1.0]);
            }

            let line_number = index + 1;

            glyph_brush.queue(VariedSection {
                screen_position: (x, y),
                text: vec![SectionText {
                    text: &line_number.to_string(),
                    // TODO: Don't hardcode scale
                    scale: Scale::uniform(40.0),
                    color: [0.4, 0.4, 0.4, 1.0],
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            });

            glyph_brush.queue(VariedSection {
                screen_position: (x + gutter_offset, y),
                text: vec![SectionText {
                    text: line,
                    scale: Scale::uniform(40.0),
                    color: [0.7, 0.7, 0.7, 1.0],
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            });

            y += 40.0;
        }

        // gutter color
        rect_brush.queue_rectangle(
            0,
            0,
            (gutter_offset - 5.0) as i32,
            size.height as i32,
            [0.06, 0.06, 0.06, 1.0],
        );
    }
}

struct Editor {
    buffers: Vec<Buffer>,
    active_buffer: usize,
}

impl Editor {
    fn new() -> Editor {
        Editor {
            buffers: vec![Buffer::new()],
            active_buffer: 0,
        }
    }

    fn draw(
        &self,
        size: PhysicalSize,
        glyph_brush: &mut GlyphBrush<()>,
        rect_brush: &mut RectangleBrush,
    ) {
        self.buffers[self.active_buffer].draw(size, glyph_brush, rect_brush);
    }

    fn scroll(&mut self, delta: f32) {
        self.buffers[self.active_buffer].scroll(delta);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("brewcode")
        .build(&event_loop)?;
    let mut size = window.inner_size().to_physical(window.hidpi_factor());
    let surface = wgpu::Surface::create(&window);

    let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        backends: wgpu::BackendBit::PRIMARY,
    })
    .expect("Failed to create adapter");

    let (mut device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    });

    // TODO: Select supported render format instead of hard-coding.
    let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;

    let mut swap_chain = device.create_swap_chain(
        &surface,
        &wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: render_format,
            width: size.width.round() as u32,
            height: size.height.round() as u32,
            present_mode: wgpu::PresentMode::Vsync,
        },
    );

    // TODO: Dynamically load fonts or something?
    let inconsolata: &[u8] =
        include_bytes!("/Users/connor/Library/Fonts/InconsolataGo-Regular.ttf");
    let mut glyph_brush =
        GlyphBrushBuilder::using_font_bytes(inconsolata).build(&mut device, render_format);

    let mut rectangle_brush = RectangleBrush::new(&device, render_format);

    window.request_redraw();

    let mut editor = Editor::new();
    let mut last_frame = std::time::Instant::now();
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,

        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Q),
                            modifiers: ModifiersState { logo: true, .. },
                            ..
                        },
                    ..
                },
            ..
        } => *control_flow = ControlFlow::Exit,

        Event::WindowEvent {
            event:
                WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::PixelDelta(delta),
                    ..
                },
            ..
        } => {
            // Fix scroll direction
            // TODO: query user preferences
            editor.scroll(-delta.y as f32);
            window.request_redraw();
        }

        Event::WindowEvent {
            event: WindowEvent::Resized(new_size),
            ..
        } => {
            size = new_size.to_physical(window.hidpi_factor());

            swap_chain = device.create_swap_chain(
                &surface,
                &wgpu::SwapChainDescriptor {
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    format: render_format,
                    width: size.width.round() as u32,
                    height: size.height.round() as u32,
                    present_mode: wgpu::PresentMode::Vsync,
                },
            );

            window.request_redraw();
        }

        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            let dt = last_frame.elapsed().as_millis();
            let fps = 1.0 / ((dt as f32) / 1000.0);
            last_frame = std::time::Instant::now();

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

            let frame = swap_chain.get_next_texture();

            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color {
                        r: 0.03,
                        g: 0.03,
                        b: 0.03,
                        a: 1.0,
                    },
                }],
                depth_stencil_attachment: None,
            });

            editor.draw(size, &mut glyph_brush, &mut rectangle_brush);

            rectangle_brush.draw(
                &device,
                &mut encoder,
                &frame.view,
                (size.width, size.height),
            );

            glyph_brush.queue(Section {
                text: &format!("{:.2} fps", fps),
                screen_position: (size.width as f32 - 200.0, 5.0),
                scale: Scale::uniform(40.0),
                color: [1.0, 1.0, 1.0, 1.0],
                ..Section::default()
            });

            glyph_brush
                .draw_queued(
                    &mut device,
                    &mut encoder,
                    &frame.view,
                    size.width.round() as u32,
                    size.height.round() as u32,
                )
                .expect("Failed to draw queued text.");

            queue.submit(&[encoder.finish()]);
        }

        // Event::EventsCleared => {
        //     window.request_redraw();
        // }
        _ => *control_flow = ControlFlow::Poll,
    });
}

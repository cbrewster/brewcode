// TODO
// * Do layout before sending to wgpu_glyph so we know glyph positions

mod rectangle_brush;

use rectangle_brush::RectangleBrush;
use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder, Scale, Section, SectionText, VariedSection};
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

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

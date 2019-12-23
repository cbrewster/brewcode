use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder, Scale, SectionText, VariedSection};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

struct Editor {
    lines: Vec<String>,
}

impl Editor {
    fn new() -> Editor {
        Editor {
            lines: vec![
                "Oh how".into(),
                "the turn tables".into(),
                "have turned".into(),
                "...".into(),
                "".into(),
                "Oh how".into(),
                "the turn tables".into(),
                "have turned".into(),
                "...".into(),
                "".into(),
                "Oh how".into(),
                "the turn tables".into(),
                "have turned".into(),
                "...".into(),
                "".into(),
                "Oh how".into(),
                "the turn tables".into(),
                "have turned".into(),
                "...".into(),
                "".into(),
            ],
        }
    }

    fn draw(&self, glyph_brush: &mut GlyphBrush<()>) {
        let x = 10.0;
        let digit_count = self.lines.len().to_string().chars().count();
        let gutter_offset = 20.0 + digit_count as f32 * 20.0;
        let mut y = 10.0;

        for (index, line) in self.lines.iter().enumerate() {
            let line_number = index + 1;

            glyph_brush.queue(VariedSection {
                screen_position: (x, y),
                text: vec![SectionText {
                    text: &line_number.to_string(),
                    scale: Scale::uniform(40.0),
                    color: [0.2, 0.2, 0.2, 1.0],
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            });

            glyph_brush.queue(VariedSection {
                screen_position: (x + gutter_offset, y),
                text: vec![SectionText {
                    text: line,
                    scale: Scale::uniform(40.0),
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            });

            y += 40.0;
        }
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

    window.request_redraw();

    let editor = Editor::new();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,

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
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                }],
                depth_stencil_attachment: None,
            });

            editor.draw(&mut glyph_brush);

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

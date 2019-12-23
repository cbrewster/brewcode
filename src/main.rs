use wgpu_glyph::{GlyphBrushBuilder, Scale, Section};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

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

            glyph_brush.queue(Section {
                text: "Hello world!",
                screen_position: (10.0, 10.0),
                color: [0.0, 0.0, 0.0, 1.0],
                scale: Scale { x: 40.0, y: 40.0 },
                bounds: (size.width as f32, size.height as f32),
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
        _ => *control_flow = ControlFlow::Wait,
    });
}

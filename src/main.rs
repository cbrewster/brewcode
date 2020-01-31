// TODO List
// * Do better text layout and more easily track metrics? (helpful for hit-testing)
// * Text selections
// * Support mouse up/down/move in editor/buffer
// * Add layout functionality

mod buffer;
mod editor;
mod rectangle_brush;

use editor::Editor;
use rectangle_brush::RectangleBrush;

use wgpu_glyph::{GlyphBrushBuilder, Scale, Section};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorIcon, WindowBuilder},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let file_name = args.nth(1).expect("Must specify a file to open");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("brewcode")
        .build(&event_loop)?;
    let mut size = window.inner_size();
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
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Vsync,
        },
    );

    // TODO: Dynamically load fonts or something?
    let inconsolata: &[u8] = include_bytes!("../res/UbuntuMono-R.ttf");
    // /Users/connor/Library/Fonts/InconsolataGo-Regular.ttf");
    let mut glyph_brush =
        GlyphBrushBuilder::using_font_bytes(inconsolata).build(&mut device, render_format);

    let mut rectangle_brush = RectangleBrush::new(&device, render_format);

    window.request_redraw();
    window.set_cursor_icon(CursorIcon::Text);

    let mut editor = Editor::new(size, file_name);
    let mut last_frame = std::time::Instant::now();

    let mut modifier_pressed = false;
    let mut cursor_position: PhysicalPosition<i32> = PhysicalPosition::new(0, 0);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,

        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            if input.virtual_keycode == Some(VirtualKeyCode::LControl) {
                modifier_pressed = input.state == ElementState::Pressed;
            }
            match (input.virtual_keycode, input.modifiers) {
                // Quit
                (Some(VirtualKeyCode::Q), ModifiersState::CTRL) => {
                    *control_flow = ControlFlow::Exit
                }

                // Save
                (Some(VirtualKeyCode::S), ModifiersState::CTRL) => {
                    editor.save();
                }

                _ => {
                    editor.handle_keyboard_input(input);
                    // TODO: Only redraw is something has changed
                    window.request_redraw();
                }
            }
        }

        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter(input),
            ..
        } => {
            if !modifier_pressed {
                editor.handle_char_input(input);
                // TODO: Only redraw is something has changed
                window.request_redraw();
            }
        }

        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            cursor_position = position;
            editor.handle_mouse_move(cursor_position);
            window.request_redraw();
        }

        Event::WindowEvent {
            event: WindowEvent::MouseInput { state, button, .. },
            ..
        } => {
            editor.handle_mouse_input(button, state, cursor_position);
            window.request_redraw();
        }

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
            size = new_size;
            editor.update_size(size);

            swap_chain = device.create_swap_chain(
                &surface,
                &wgpu::SwapChainDescriptor {
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    format: render_format,
                    width: size.width,
                    height: size.height,
                    present_mode: wgpu::PresentMode::Vsync,
                },
            );

            window.request_redraw();
        }

        Event::RedrawRequested(_) => {
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
                (size.width as f64, size.height as f64),
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
                    size.width,
                    size.height,
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

use crate::buffer::Buffer;
use crate::rectangle_brush::RectangleBrush;
use wgpu_glyph::GlyphBrush;
use winit::{dpi::PhysicalSize, event::KeyboardInput};

pub struct Editor {
    buffers: Vec<Buffer>,
    active_buffer: usize,
    size: PhysicalSize,
}

impl Editor {
    pub fn new(size: PhysicalSize) -> Editor {
        Editor {
            buffers: vec![Buffer::new(size)],
            active_buffer: 0,
            size,
        }
    }

    pub fn update_size(&mut self, size: PhysicalSize) {
        self.size = size;
        for buffer in &mut self.buffers {
            buffer.update_size(size);
        }
    }

    pub fn handle_char_input(&mut self, input: char) {
        self.buffers[self.active_buffer].handle_char_input(input);
    }

    pub fn handle_keyboard_input(&mut self, input: KeyboardInput) {
        self.buffers[self.active_buffer].handle_keyboard_input(input);
    }

    pub fn draw(
        &self,
        size: PhysicalSize,
        glyph_brush: &mut GlyphBrush<()>,
        rect_brush: &mut RectangleBrush,
    ) {
        self.buffers[self.active_buffer].draw(size, glyph_brush, rect_brush);
    }

    pub fn scroll(&mut self, delta: f32) {
        self.buffers[self.active_buffer].scroll(delta);
    }
}

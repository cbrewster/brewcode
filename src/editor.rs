use crate::{
    buffer::Buffer,
    rectangle_brush::RectangleBrush,
    render::RenderContext,
    layout::LayoutContext,
};
use wgpu_glyph::GlyphBrush;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyboardInput, MouseButton},
};

pub struct Editor {
    buffers: Vec<Buffer>,
    active_buffer: usize,
    size: PhysicalSize<u32>,
}

impl Editor {
    pub fn new(size: PhysicalSize<u32>, file_name: String) -> Self {
        Self {
            buffers: vec![Buffer::new(size, file_name)],
            active_buffer: 0,
            size,
        }
    }

    pub fn save(&self) {
        self.buffers[self.active_buffer].save();
    }

    pub fn update_size(&mut self, size: PhysicalSize<u32>) {
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

    pub fn handle_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
        position: PhysicalPosition<i32>,
    ) {
        self.buffers[self.active_buffer].handle_mouse_input(button, state, position);
    }

    pub fn handle_mouse_move(&mut self, position: PhysicalPosition<i32>) {
        self.buffers[self.active_buffer].handle_mouse_move(position);
    }

    pub fn layout(&mut self, context: &mut LayoutContext) {
        self.buffers[self.active_buffer].layout(context);
    }

    pub fn render(&self, context: &mut RenderContext) {
        self.buffers[self.active_buffer].render(context);
    }

    pub fn scroll(&mut self, delta: f32) {
        self.buffers[self.active_buffer].scroll(delta);
    }
}

use crate::rectangle_brush::RectangleBrush;
use wgpu_glyph::{GlyphBrush, VariedSection, Font};

pub struct RenderContext<'a> {
    /// The size available for rendering
    pub size: (f32, f32),
    pub glyph_brush: &'a mut GlyphBrush<'static, ()>,
    pub rectangle_brush: &'a mut RectangleBrush,
}

impl RenderContext<'_> {
    pub fn draw_text(&mut self, section: VariedSection) {
        self.glyph_brush.queue(section);
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: [f32; 4]) {
        self.rectangle_brush.queue_rectangle(x, y, width, height, color);
    }

    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    pub fn font(&self) -> &Font {
        &self.glyph_brush.fonts()[0]
    }
}

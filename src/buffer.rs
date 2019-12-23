use crate::rectangle_brush::RectangleBrush;
use wgpu_glyph::{GlyphBrush, Scale, SectionText, VariedSection};
use winit::dpi::PhysicalSize;

pub struct Buffer {
    lines: Vec<String>,
    scroll: f32,
}

impl Buffer {
    pub fn new() -> Buffer {
        let file = include_str!("main.rs");
        Buffer {
            scroll: 0.0,
            lines: file.lines().map(|line| line.to_owned()).collect(),
        }
    }

    pub fn scroll(&mut self, delta: f32) {
        let max_scroll = if self.lines.is_empty() {
            0.0
        } else {
            // TODO: Find better way to calculate max scroll based on line count
            ((self.lines.len() - 1) as f32 * 40.0) + 5.0
        };

        self.scroll = (self.scroll + delta).max(0.0).min(max_scroll);
    }

    pub fn draw(
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

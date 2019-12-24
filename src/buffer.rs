use crate::rectangle_brush::RectangleBrush;
use std::path::{Path, PathBuf};
use wgpu_glyph::{GlyphBrush, Point, Scale, SectionText, VariedSection};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyboardInput, VirtualKeyCode},
};

const SCALE: f32 = 40.0;

struct Cursor {
    row: usize,
    col: usize,
    col_affinity: usize,
}

pub struct Buffer {
    lines: Vec<String>,
    scroll: f32,
    cursor: Cursor,
    size: PhysicalSize,
    path: PathBuf,
}

impl Buffer {
    pub fn new(size: PhysicalSize, file_name: String) -> Buffer {
        let path = Path::new(&file_name);
        let file = std::fs::read_to_string(path).expect("Failed to read file.");
        // TODO: Not sure if just splitting '\n' is right here.
        // I was using lines, but the trailing empty newline was omitted by lines.
        let mut lines: Vec<String> = file.split('\n').map(|line| line.to_owned()).collect();
        if lines.len() == 0 {
            lines.push(String::new());
        }
        Buffer {
            scroll: 0.0,
            lines,
            cursor: Cursor {
                row: 0,
                col: 0,
                col_affinity: 0,
            },
            size,
            path: path.into(),
        }
    }

    pub fn save(&self) {
        std::fs::write(&self.path, self.lines.join("\n")).expect("Failed to save file.");
    }

    pub fn update_size(&mut self, size: PhysicalSize) {
        self.size = size;
    }

    fn ensure_cursor_in_view(&mut self) {
        let cursor_y = self.cursor.row as f32 * SCALE;
        let bottom = self.scroll + self.size.height as f32;

        if cursor_y < self.scroll {
            self.scroll = cursor_y;
        } else if cursor_y + SCALE > bottom {
            self.scroll = cursor_y - self.size.height as f32 + SCALE + 5.0;
        }
    }

    pub fn scroll(&mut self, delta: f32) {
        let max_scroll = if self.lines.is_empty() {
            0.0
        } else {
            // TODO: Find better way to calculate max scroll based on line count
            ((self.lines.len() - 1) as f32 * SCALE) + 5.0
        };

        self.scroll = (self.scroll + delta).max(0.0).min(max_scroll);
    }

    pub fn handle_click(&mut self, position: PhysicalPosition) {
        // TODO: this is duplicated in draw
        let x_pad = 10.0;
        let digit_count = self.lines.len().to_string().chars().count();
        let gutter_offset = x_pad + 30.0 + digit_count as f32 * (SCALE / 2.0);

        let abs_position = PhysicalPosition::new(
            (position.x - gutter_offset as f64).max(0.0),
            position.y + self.scroll as f64,
        );
        let line = (abs_position.y / 40.0).floor() as usize;
        if line >= self.lines.len() {
            self.cursor.row = self.lines.len() - 1;
            self.cursor.col = self.lines.last().unwrap().len();
        } else {
            // TODO: HACK this should not be hardcoded
            let h_advance = 19.065777;
            let col = (abs_position.x / h_advance).round() as usize;
            self.cursor.row = line;
            self.cursor.col = col.min(self.lines[line].len());
        }
        self.cursor.col_affinity = self.cursor.col;
    }

    pub fn handle_char_input(&mut self, input: char) {
        if input == '\n' || input == '\r' {
            let new_line = self.lines[self.cursor.row].split_off(self.cursor.col);
            self.cursor.row += 1;
            self.lines.insert(self.cursor.row, new_line);
            self.cursor.col = 0;
        } else if input == 127 as char {
            if self.cursor.col > 0 {
                self.lines[self.cursor.row].remove(self.cursor.col - 1);
                self.cursor.col -= 1;
            } else if self.cursor.row > 0 {
                let remaining = self.lines.remove(self.cursor.row);
                self.cursor.row -= 1;
                self.cursor.col = self.lines[self.cursor.row].len();
                self.lines[self.cursor.row].push_str(&remaining);
            }
        } else {
            self.lines[self.cursor.row].insert(self.cursor.col, input);
            self.cursor.col += 1;
        }
        self.cursor.col_affinity = self.cursor.col;
        self.ensure_cursor_in_view();
    }

    pub fn handle_keyboard_input(&mut self, input: KeyboardInput) {
        let keycode = match input.virtual_keycode {
            Some(keycode) => keycode,
            None => return,
        };

        if input.state == ElementState::Released {
            return;
        }

        match keycode {
            VirtualKeyCode::Up => {
                self.cursor.row = (self.cursor.row as isize - 1)
                    .max(0)
                    .min(self.lines.len() as isize) as usize;
                self.cursor.col = self.lines[self.cursor.row]
                    .len()
                    .min(self.cursor.col_affinity);
            }
            VirtualKeyCode::Down => {
                self.cursor.row = (self.cursor.row as isize + 1)
                    .max(0)
                    .min(self.lines.len() as isize - 1) as usize;
                self.cursor.col = self.lines[self.cursor.row]
                    .len()
                    .min(self.cursor.col_affinity);
            }
            VirtualKeyCode::Left => {
                if self.cursor.col == 0 {
                    if self.cursor.row > 0 {
                        self.cursor.row -= 1;
                        self.cursor.col = self.lines[self.cursor.row].len();
                    }
                } else {
                    self.cursor.col -= 1;
                }
                self.cursor.col_affinity = self.cursor.col;
            }
            VirtualKeyCode::Right => {
                if self.cursor.col >= self.lines[self.cursor.row].len() {
                    if self.cursor.row < self.lines.len() - 1 {
                        self.cursor.row += 1;
                        self.cursor.col = 0;
                    }
                } else {
                    self.cursor.col += 1;
                }
                self.cursor.col_affinity = self.cursor.col;
            }
            _ => {}
        }
        self.ensure_cursor_in_view();
    }

    pub fn draw(
        &self,
        size: PhysicalSize,
        glyph_brush: &mut GlyphBrush<()>,
        rect_brush: &mut RectangleBrush,
    ) {
        let x_pad = 10.0;
        let digit_count = self.lines.len().to_string().chars().count();
        let gutter_offset = x_pad + 30.0 + digit_count as f32 * (SCALE / 2.0);
        let mut y = 5.0 - self.scroll;

        // gutter color
        rect_brush.queue_rectangle(
            0,
            0,
            (digit_count as f32 * (SCALE / 2.0) + x_pad * 2.0) as i32,
            size.height as i32,
            [0.06, 0.06, 0.06, 1.0],
        );

        for (index, line) in self.lines.iter().enumerate() {
            if y < -SCALE {
                y += SCALE;
                continue;
            }
            if y > size.height as f32 {
                break;
            }

            let mut line_no_color = [0.4, 0.4, 0.4, 1.0];

            if index == self.cursor.row {
                line_no_color = [1.0, 1.0, 1.0, 1.0];

                let mut layout = glyph_brush.fonts().first().unwrap().layout(
                    line,
                    Scale::uniform(SCALE),
                    Point { x: 0.0, y: 0.0 },
                );
                let mut x_pos = 0.0;
                for _ in 0..self.cursor.col {
                    let positioned_glyph = layout.next().unwrap();
                    x_pos += positioned_glyph.unpositioned().h_metrics().advance_width;
                }

                let cursor_x = gutter_offset + x_pos;
                // active line
                rect_brush.queue_rectangle(
                    0,
                    y as i32,
                    size.width as i32,
                    SCALE as i32,
                    [1.0, 1.0, 1.0, 0.05],
                );

                rect_brush.queue_rectangle(
                    cursor_x as i32,
                    y as i32,
                    4,
                    SCALE as i32,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }

            let line_number = index + 1;

            glyph_brush.queue(VariedSection {
                screen_position: (x_pad, y),
                text: vec![SectionText {
                    text: &line_number.to_string(),
                    // TODO: Don't hardcode scale
                    scale: Scale::uniform(SCALE),
                    color: line_no_color,
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            });

            glyph_brush.queue(VariedSection {
                screen_position: (gutter_offset, y),
                text: vec![SectionText {
                    text: line,
                    scale: Scale::uniform(SCALE),
                    color: [0.7, 0.7, 0.7, 1.0],
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            });

            y += SCALE;
        }
    }
}

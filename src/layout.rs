use wgpu_glyph::Font;

// TODO: Add more sophisticated rendering, kinda like flutter.
// Maybe continue some of the work I did in Imagine but with new knowledge!

pub struct LayoutContext<'a> {
    /// The size available for layout?
    pub size: (f32, f32),
    // Probably should be broader than 'static?
    // Font may be cheap to clone, so maybe just own the Font here?
    pub font: Font<'a>,
}

impl LayoutContext<'_> {
    pub fn size(&self) -> (f32, f32) {
        self.size
    }
}

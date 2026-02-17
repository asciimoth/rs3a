use crate::helpers::escape_html;

/// Represents font properties used for rendering 3a art to SVG or similar.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Font {
    // The font family name (e.g., "Courier New").
    pub family: String,
    /// The font size in pixels.
    pub size: usize,
    /// The width of each character cell in pixels.
    pub width: usize,
    /// The height of each character cell in pixels.
    pub height: usize,
    /// Horizontal offset for foreground rendering.
    pub fg_offset_x: usize,
    /// Vertical offset for foreground rendering.
    pub fg_offset_y: usize,
}

impl Font {
    /// Generates a CSS style block containing the font family and size.
    pub fn to_style(&self) -> String {
        format!(
            "<style>\ntext {{ font-family: \"{}\", monospace; font-size:{}px; }}\n</style>\n",
            escape_html(self.family.as_str()),
            self.size,
        )
    }
}

impl Default for Font {
    /// Provides default font settings: "Courier New", size 20,
    /// cell size 12x20, offsets (0,2).
    fn default() -> Self {
        Self {
            family: "Courier New".into(),
            size: 20,
            width: 12,
            height: 20,
            fg_offset_x: 0,
            fg_offset_y: 2,
        }
    }
}

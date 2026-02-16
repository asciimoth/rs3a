use crate::helpers::escape_html;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Font {
    pub family: String,
    pub size: usize,
    pub width: usize,
    pub height: usize,
    pub fg_offset_x: usize,
    pub fg_offset_y: usize,
}

impl Font {
    pub fn to_style(&self) -> String {
        format!(
            "<style>\ntext {{ font-family: \"{}\", monospace; font-size:{}px; }}\n</style>\n",
            escape_html(self.family.as_str()),
            self.size,
        )
    }
}

impl Default for Font {
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

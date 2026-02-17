use core::fmt;
use std::{collections::HashMap, str::FromStr};

use ordermap::OrderMap;

use crate::{chars::Char, comments::Comments, error::Error};

/// The four-bit ANSI color set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color4 {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}


/// Represents a color in the 3a format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// No color / terminal default color
    None,
    /// 4-bit color with brightness
    Color4(Color4, bool),
    /// 256-color
    Color256(u8),
    // RGB
    RGB(u8, u8, u8),
}

impl Color {
    /// Creates a color from a built-in character mapping (0-9a-f) to 4-bit colors.
    pub fn from_char_builtin(c: Char) -> Self {
        match c.char {
            '0' => Self::Color4(Color4::Black, false),
            '1' => Self::Color4(Color4::Red, false),
            '2' => Self::Color4(Color4::Green, false),
            '3' => Self::Color4(Color4::Yellow, false),
            '4' => Self::Color4(Color4::Blue, false),
            '5' => Self::Color4(Color4::Magenta, false),
            '6' => Self::Color4(Color4::Cyan, false),
            '7' => Self::Color4(Color4::White, false),

            '8' => Self::Color4(Color4::Black, true),
            '9' => Self::Color4(Color4::Red, true),
            'a' => Self::Color4(Color4::Green, true),
            'b' => Self::Color4(Color4::Yellow, true),
            'c' => Self::Color4(Color4::Blue, true),
            'd' => Self::Color4(Color4::Magenta, true),
            'e' => Self::Color4(Color4::Cyan, true),
            'f' => Self::Color4(Color4::White, true),

            _ => Self::None,
        }
    }
}

/// Returns the default color (None).
impl Default for Color {
    fn default() -> Self {
        Self::None
    }
}

/// Parses a color from a string: color names ("red", "bright-green"),
/// 256-color index (0-255), or hex RGB ("rrggbb").
impl FromStr for Color {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();
        match s.as_str() {
            "black" => Ok(Self::Color4(Color4::Black, false)),
            "red" => Ok(Self::Color4(Color4::Red, false)),
            "green" => Ok(Self::Color4(Color4::Green, false)),
            "yellow" => Ok(Self::Color4(Color4::Yellow, false)),
            "blue" => Ok(Self::Color4(Color4::Blue, false)),
            "magenta" => Ok(Self::Color4(Color4::Magenta, false)),
            "cyan" => Ok(Self::Color4(Color4::Cyan, false)),
            "white" => Ok(Self::Color4(Color4::White, false)),

            "bright-black" => Ok(Self::Color4(Color4::Black, true)),
            "gray" => Ok(Self::Color4(Color4::Black, true)),
            "grey" => Ok(Self::Color4(Color4::Black, true)),
            "bright-red" => Ok(Self::Color4(Color4::Red, true)),
            "bright-green" => Ok(Self::Color4(Color4::Green, true)),
            "bright-yellow" => Ok(Self::Color4(Color4::Yellow, true)),
            "bright-blue" => Ok(Self::Color4(Color4::Blue, true)),
            "bright-magenta" => Ok(Self::Color4(Color4::Magenta, true)),
            "bright-cyan" => Ok(Self::Color4(Color4::Cyan, true)),
            "bright-white" => Ok(Self::Color4(Color4::White, true)),

            s => match s.parse::<u8>() {
                Ok(c) => Ok(Self::Color256(c)),
                Err(_) => {
                    let err = Error::ColorParsing(String::from(s));
                    if s.len() != 6 {
                        return Err(err);
                    }
                    let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| err.clone())?;
                    let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| err.clone())?;
                    let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| err.clone())?;
                    Ok(Self::RGB(r, g, b))
                }
            },
        }
    }
}

/// Formats the color as a string (color name, index, or hex RGB).
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Color::None => write!(f, ""),
            Color::Color4(c, b) => {
                let mut prefix = "";
                if *b {
                    prefix = "bright-";
                };
                write!(
                    f,
                    "{}{}",
                    prefix,
                    match c {
                        Color4::Black => "black",
                        Color4::Red => "red",
                        Color4::Green => "green",
                        Color4::Yellow => "yellow",
                        Color4::Blue => "blue",
                        Color4::Magenta => "magenta",
                        Color4::Cyan => "cyan",
                        Color4::White => "white",
                    }
                )
            }
            Color::Color256(c) => write!(f, "{}", c),
            Color::RGB(r, g, b) => write!(f, "{:02x}{:02x}{:02x}", r, g, b),
        }
    }
}

impl Color {
    /// Return an ANSI SGR escape sequence for this color.
    ///
    /// If `is_fg` is true, returns a foreground color sequence (uses `38` / 30–97 codes).
    /// If `is_fg` is false, returns a background color sequence (uses `48` / 40–107 codes).
    pub fn to_ansi(&self, is_fg: bool) -> String {
        match self {
            Color::None => {
                let code = if is_fg { 39 } else { 49 };
                format!("\x1b[{}m", code)
            }
            Color::Color4(col, bright) => {
                // base index 0..7 maps to black..white
                let idx = match col {
                    Color4::Black => 0,
                    Color4::Red => 1,
                    Color4::Green => 2,
                    Color4::Yellow => 3,
                    Color4::Blue => 4,
                    Color4::Magenta => 5,
                    Color4::Cyan => 6,
                    Color4::White => 7,
                };
                if *bright {
                    // Bright 4-bit colors: 90-97 fg, 100-107 bg
                    let code = if is_fg { 90 + idx } else { 100 + idx };
                    format!("\x1b[{}m", code)
                } else {
                    // Normal 4-bit colors: 30-37 fg, 40-47 bg
                    let code = if is_fg { 30 + idx } else { 40 + idx };
                    format!("\x1b[{}m", code)
                }
            }

            Color::Color256(n) => {
                // 256-color: 38;5;<n> (fg) or 48;5;<n> (bg)
                let prefix = if is_fg { "38" } else { "48" };
                format!("\x1b[{};5;{}m", prefix, n)
            }

            Color::RGB(r, g, b) => {
                // Truecolor: 38;2;R;G;B (fg) or 48;2;R;G;B (bg)
                let prefix = if is_fg { "38" } else { "48" };
                format!("\x1b[{};2;{};{};{}m", prefix, r, g, b)
            }
        }
    }
}

/// A pair of foreground and background colors.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorPair {
    pub fg: Color,
    pub bg: Color,
}

impl ColorPair {
    /// Returns a new ColorPair with foreground and background swapped.
    pub fn invert(&self) -> Self {
        Self {
            fg: self.bg,
            bg: self.fg,
        }
    }
    /// Returns combined ANSI escape sequences for both foreground and background colors.
    pub fn to_ansi(&self) -> String {
        return self.fg.to_ansi(true) + self.bg.to_ansi(false).as_str();
    }
    /// Returns ANSI escape sequences only if this pair differs from the previous one; otherwise returns empty string.
    pub fn to_ansi_rel(&self, prev: &Option<Self>) -> String {
        if Some(*self) != *prev {
            self.to_ansi()
        } else {
            "".into()
        }
    }
    /// Creates a color pair from a built-in character mapping.
    pub fn from_char_builtin(c: Char) -> Self {
        Self {
            fg: Color::from_char_builtin(c),
            bg: Color::None,
        }
    }
}

/// Formats the color pair as "fg:color bg:color" or just one if the other is None.
impl fmt::Display for ColorPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.fg == Color::None && self.bg == Color::None {
            return Ok(());
        }
        if self.fg == Color::None {
            return write!(f, "bg:{}", self.bg);
        }
        if self.bg == Color::None {
            return write!(f, "fg:{}", self.fg);
        }
        write!(f, "fg:{} bg:{}", self.fg, self.bg)
    }
}

/// Parses a color pair from a string like "fg:red bg:blue".
impl FromStr for ColorPair {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pair = Self::default();
        let mut fg_oc = false;
        let mut bg_oc = false;
        for ss in s.split(" ") {
            let ss = ss.trim();
            if ss.is_empty() {
                continue;
            }
            if let Some(fgs) = ss.strip_prefix("fg:") {
                if fg_oc {
                    return Err(Error::ColorDuplicate(String::from("fg"), String::from(s)));
                }
                pair.fg = fgs.parse::<Color>()?;
                fg_oc = true;
                continue;
            }
            if let Some(bgs) = ss.strip_prefix("bg:") {
                if bg_oc {
                    return Err(Error::ColorDuplicate(String::from("bg"), String::from(s)));
                }
                pair.bg = bgs.parse::<Color>()?;
                bg_oc = true;
                continue;
            }
            return Err(Error::ColorParsing(String::from(s)));
        }
        Ok(pair)
    }
}


/// A mapping from character codes to color pairs, with optional comments per entry.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Palette {
    pub palette: OrderMap<Char, (ColorPair, Comments)>,
}

impl Palette {
    /// Removes all comments from the palette entries, keeping only color pairs.
    pub fn strip_comments(&mut self) {
        let keys: Vec<Char> = self.palette.keys().map(|k| k.clone()).collect();
        for key in keys {
            if let Some((pair, _)) = self.palette.get(&key) {
                self.palette.insert(key, (*pair, Vec::new()));
            }
        }
    }
    /// Returns the number of entries in the palette.
    pub fn len(&self) -> usize {
        self.palette.len()
    }
    /// Searches for a color pair in the palette and returns its character code if found.
    pub fn search_color(&self, col: ColorPair) -> Option<Char> {
        for (k, v) in &self.palette {
            if v.0 == col {
                return Some(*k);
            }
        }
        None
    }
    /// Checks if a character code is defined in the palette.
    pub fn contains_color(&self, name: Char) -> bool {
        self.palette.contains_key(&name)
    }
    /// Returns the color pair for a character code, falling back to built-in mapping if not found.
    pub fn get_color(&self, name: Char) -> ColorPair {
        if let Some((pair, _)) = self.palette.get(&name) {
            *pair
        } else {
            ColorPair::from_char_builtin(name)
        }
    }
    /// Sets the color pair for a character code;
    /// if it matches the built-in mapping, the entry is removed.
    pub fn set_color(&mut self, name: Char, col: ColorPair) {
        if ColorPair::from_char_builtin(name) == col {
            self.palette.remove(&name);
        } else {
            self.palette.insert(name, (col, Vec::new()));
        }
    }
    /// Removes the entry for a character code from the palette.
    pub fn remove_color(&mut self, name: Char) {
        self.palette.remove(&name);
    }
    pub(crate) fn add_parsing_color(
        &mut self,
        name: Char,
        pair: ColorPair,
        comments: Vec<String>,
    ) -> Result<(), Error> {
        if self.palette.contains_key(&name) {
            return Err(Error::ColorMapDup(name.into()));
        }
        self.palette.insert(name, (pair, comments));
        Ok(())
    }
}

/// Formats the palette as `col <char> <colorpair>` lines,
/// with optional comment lines prefixed by ";;".
impl fmt::Display for Palette {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, mapping) in &self.palette {
            for c in &mapping.1 {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "col {} {}", name, mapping.0)?;
        }
        Ok(())
    }
}

/// A mapping from colors to CSS color strings, used for SVG output.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct CSSColorMap {
    pub map: HashMap<(Color, bool), String>,
}

impl CSSColorMap {
    /// Returns the CSS color string for an optional color and foreground flag; uses default if None.
    pub fn map_opt(&self, color: Option<Color>, foreground: bool) -> String {
        let color = if let Some(color) = color {
            color
        } else {
            Color::None
        };
        self.map(color, foreground)
    }
    /// Returns the CSS color string for a color and foreground flag; uses built-in defaults if not mapped
    pub fn map(&self, color: Color, foreground: bool) -> String {
        if let Some(s) = self.map.get(&(color, foreground)) {
            s.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '#')
                .collect()
        } else {
            match (color, foreground) {
                // No color aka default color
                (Color::None, true) => "#ffffff".into(),
                (Color::None, false) => "#000000".into(),
                // 4-bit ansi color name and bright flag
                (Color::Color4(Color4::Black, false), _) => "#000000".into(),
                (Color::Color4(Color4::Black, true), _) => "#4e4e4e".into(),
                (Color::Color4(Color4::Red, false), _) => "#800000".into(),
                (Color::Color4(Color4::Red, true), _) => "#ff0000".into(),
                (Color::Color4(Color4::Green, false), _) => "#008000".into(),
                (Color::Color4(Color4::Green, true), _) => "#00ff00".into(),
                (Color::Color4(Color4::Yellow, false), _) => "#808000".into(),
                (Color::Color4(Color4::Yellow, true), _) => "#ffff00".into(),
                (Color::Color4(Color4::Blue, false), _) => "#000080".into(),
                (Color::Color4(Color4::Blue, true), _) => "#0000ff".into(),
                (Color::Color4(Color4::Magenta, false), _) => "#800080".into(),
                (Color::Color4(Color4::Magenta, true), _) => "#ff00ff".into(),
                (Color::Color4(Color4::Cyan, false), _) => "#008080".into(),
                (Color::Color4(Color4::Cyan, true), _) => "#00ffff".into(),
                (Color::Color4(Color4::White, false), _) => "#c0c0c0".into(),
                (Color::Color4(Color4::White, true), _) => "#ffffff".into(),
                // 8-bit ansi color
                (Color::Color256(c), _) => {
                    let c = c as usize;
                    // first 16 are the standard/system colors
                    let table16 = [
                        "#000000", "#800000", "#008000", "#808000", "#000080", "#800080",
                        "#008080", "#c0c0c0", "#4e4e4e", "#ff0000", "#00ff00", "#ffff00",
                        "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
                    ];
                    if c < 16 {
                        table16[c].to_string()
                    } else if c < 232 {
                        // 6x6x6 color cube
                        let idx = c - 16;
                        let r = idx / 36;
                        let g = (idx % 36) / 6;
                        let b = idx % 6;
                        let levels: [u8; 6] = [0, 95, 135, 175, 215, 255];
                        format!("#{:02x}{:02x}{:02x}", levels[r], levels[g], levels[b])
                    } else {
                        // grayscale ramp: 232..255 -> 24 shades
                        let gray = 8 + (c - 232) * 10;
                        format!("#{:02x}{:02x}{:02x}", gray, gray, gray)
                    }
                }
                (Color::RGB(r, g, b), _) => format!("#{:02x}{:02x}{:02x}", r, g, b),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color4_basic_fg_bg() {
        assert_eq!(
            Color::Color4(Color4::Black, false).to_ansi(true),
            "\x1b[30m"
        );
        assert_eq!(
            Color::Color4(Color4::White, false).to_ansi(false),
            "\x1b[47m"
        );
        // bright versions
        assert_eq!(Color::Color4(Color4::Red, true).to_ansi(true), "\x1b[91m");
        assert_eq!(
            Color::Color4(Color4::Blue, true).to_ansi(false),
            "\x1b[104m"
        );
    }

    #[test]
    fn test_color256_sequences() {
        assert_eq!(Color::Color256(0).to_ansi(true), "\x1b[38;5;0m");
        assert_eq!(Color::Color256(199).to_ansi(true), "\x1b[38;5;199m");
        assert_eq!(Color::Color256(255).to_ansi(false), "\x1b[48;5;255m");
    }

    #[test]
    fn test_rgb_sequences() {
        assert_eq!(Color::RGB(10, 20, 30).to_ansi(true), "\x1b[38;2;10;20;30m");
        assert_eq!(
            Color::RGB(255, 128, 0).to_ansi(false),
            "\x1b[48;2;255;128;0m"
        );
    }

    #[test]
    fn test_all_color4_indices() {
        // ensure mapping order is correct for non-bright fg (30..37)
        let expected = [
            "\x1b[30m", // Black
            "\x1b[31m", // Red
            "\x1b[32m", // Green
            "\x1b[33m", // Yellow
            "\x1b[34m", // Blue
            "\x1b[35m", // Magenta
            "\x1b[36m", // Cyan
            "\x1b[37m", // White
        ];

        let colors = [
            Color4::Black,
            Color4::Red,
            Color4::Green,
            Color4::Yellow,
            Color4::Blue,
            Color4::Magenta,
            Color4::Cyan,
            Color4::White,
        ];

        for (c, &exp) in colors.iter().zip(expected.iter()) {
            assert_eq!(Color::Color4(c.clone(), false).to_ansi(true), exp);
        }
    }

    #[test]
    fn test_none_color_resets() {
        assert_eq!(Color::None.to_ansi(true), "\x1b[39m");
        assert_eq!(Color::None.to_ansi(false), "\x1b[49m");
    }
}

pub(crate) fn trans_color(leacy: char) -> char {
    match leacy {
        '0' => '0',
        '1' => '4',
        '2' => '2',
        '3' => '6',
        '4' => '1',
        '5' => '5',
        '6' => '3',
        '7' => '7',
        '8' => '8',
        '9' => 'c',
        'a' => 'a',
        'b' => 'e',
        'c' => '9',
        'd' => 'd',
        'e' => 'b',
        'f' => 'f',
        _ => '_',
    }
}

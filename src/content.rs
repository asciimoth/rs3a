use core::fmt;
use std::io::{self, BufReader, Read};

use crate::{
    chars::{normalize_text, Char, SPACE, UNDERSCORE},
    colors::{trans_color, CSSColorMap, ColorPair, Palette},
    delay::Delay,
    error::{Error, Result},
    font::Font,
    header::{Header, LegacyColorMode, LegacyHeaderInfo},
    helpers::{escape_html, timing_for_svg},
    Color,
};

/// A single cell in a frame, containing a text character
/// and an optional color character.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Cell {
    /// The text character displayed in this cell.
    pub text: Char,
    /// The color character (if any) that maps to a palette color pair.
    pub color: Option<Char>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            text: SPACE,
            color: None,
        }
    }
}

impl Cell {
    /// Returns true if this cell has a color assigned.
    pub fn color(&self) -> bool {
        self.color != None
    }

    /// Converts the cell's color to a `ColorPair` using the given palette.
    pub fn to_pair(&self, palette: &Palette) -> ColorPair {
        if let Some(color) = self.color {
            palette.get_color(color)
        } else {
            ColorPair::default()
        }
    }

    /// Returns the ANSI escape sequence for this cell.
    pub fn ansi(&self, palette: &Palette) -> String {
        if let Some(color) = self.color {
            format!(
                "{}{}{}",
                palette.get_color(color).to_ansi(),
                self.text,
                ColorPair::default().to_ansi(),
            )
        } else {
            self.text.into()
        }
    }
}

/// A single frame of 3a art, consisting of a grid of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub(crate) color: usize,
    pub(crate) width: usize,
    pub(crate) rows: Vec<Vec<Cell>>,
}

/// Merges a text frame and a color frame into a single frame.
/// Returns an error if dimensions mismatch.
pub fn merge_frames(text: &Frame, color: &Frame) -> Result<Frame> {
    if text.height() != color.height() {
        return Err(Error::HeightMismatch);
    }
    if text.width() != color.width() {
        return Err(Error::WidthMismatch);
    }
    let mut frame = Frame::new(text.width(), text.height(), Cell::default());
    for r in 0..frame.height() {
        for c in 0..frame.width() {
            frame.rows[r][c] = Cell {
                text: text.rows[r][c].text,
                color: color.rows[r][c].color,
            };
        }
    }
    frame.recalc_colors();
    Ok(frame)
}

// SVG
impl Frame {
    /// Generates SVG background rectangles for colored cells.
    pub fn to_svg_frame_bg(&self, palette: &Palette, map: &CSSColorMap, font: &Font) -> String {
        let mut txt = String::new();
        for r in 0..self.height() {
            for c in 0..self.width() {
                if let Some(name) = self.rows[r][c].color {
                    let bg = palette.get_color(name).bg;
                    if bg == Color::None {
                        continue;
                    }
                    let fill = map.map(bg, false);
                    let x = font.width * c;
                    let y = font.height * r;
                    // TODO: Optimise sequences
                    txt += &format!(
                        "<rect x=\"{}\"  y=\"{}\"  width=\"{}\" height=\"{}\" fill=\"{}\"/>\n",
                        x, y, font.width, font.height, fill
                    );
                };
            }
        }
        txt
    }

    /// Generates SVG text with foreground colors.
    pub fn to_svg_frame_txt_fg(&self, palette: &Palette, map: &CSSColorMap, font: &Font) -> String {
        let mut txt =
            "<text x=\"0\" y=\"0\" xml:space=\"preserve\" dominant-baseline=\"hanging\">\n".into();
        for r in 0..self.height() {
            for c in 0..self.width() {
                let fg = if let Some(name) = self.rows[r][c].color {
                    Some(palette.get_color(name).fg)
                } else {
                    None
                };
                let fill = map.map_opt(fg, true);
                let x = font.width * c + font.fg_offset_x;
                let y = font.height * r + font.fg_offset_y;
                // TODO: Optimise sequences
                let span = format!(
                    "<tspan x=\"{}\" y=\"{}\" fill=\"{}\">{}</tspan>\n",
                    x,
                    y,
                    fill,
                    escape_html(&self.rows[r][c].text.to_string()),
                );
                txt += span.as_str();
            }
        }
        txt += "</text>\n";
        txt
    }

    /// Generates SVG text without colors.
    pub fn to_svg_frame_txt(&self, font: &Font) -> String {
        let mut txt =
            "<text x=\"0\" y=\"0\" xml:space=\"preserve\" dominant-baseline=\"hanging\">\n".into();
        for r in 0..self.height() {
            let mut row = String::new();
            for c in 0..self.width() {
                row += self.rows[r][c].text.to_string().as_str();
            }
            let x = font.fg_offset_x;
            let y = font.height * r + font.fg_offset_y;
            let row = format!(
                "<tspan x=\"{}\" y=\"{}\">{}</tspan>\n",
                x,
                y,
                escape_html(&row)
            );
            txt += row.as_str();
        }
        txt += "</text>\n";
        txt
    }

    /// Generates SVG for this frame, optionally with colors.
    pub fn to_svg_frame(
        &self,
        colors: bool,
        palette: &Palette,
        map: &CSSColorMap,
        font: &Font,
    ) -> String {
        if colors {
            self.to_svg_frame_bg(palette, map, font) + &self.to_svg_frame_txt_fg(palette, map, font)
        } else {
            self.to_svg_frame_txt(font)
        }
    }

    /// Generates a complete SVG document for this frame.
    pub fn to_svg(
        &self,
        colors: bool,
        palette: &Palette,
        map: &CSSColorMap,
        font: &Font,
    ) -> String {
        let mut svg = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n".into();
        let width = self.width() * font.width;
        let height = self.height() * font.height;
        svg += format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" role=\"img\">\n",
            width, height, width, height
        )
        .as_str();
        svg += font.to_style().as_str();
        if colors {
            svg += format!(
                "<rect x=\"0\"  y=\"0\"  width=\"{}\" height=\"{}\" fill=\"{}\"/>\n",
                width,
                height,
                map.map(Color::None, false)
            )
            .as_str();
        }
        svg += self.to_svg_frame(colors, palette, map, font).as_str();
        svg += "</svg>\n";
        svg
    }
}

impl Frame {
    /// Reads a color-only frame from input lines.
    pub fn read_color<R: Read>(lines: &mut io::Lines<BufReader<R>>) -> Result<Self> {
        let mut width: usize = 0;
        let mut rows: Vec<Vec<Cell>> = Vec::new();
        let mut color: usize = 0;
        for line in lines {
            let line = normalize_text(line?.as_str());
            if line.is_empty() {
                break;
            }
            let full_line: Vec<char> = line.chars().collect();
            if width != 0 && full_line.len() != width {
                return Err(Error::WidthMismatch);
            }
            width = full_line.len();
            let mut row: Vec<Cell> = Vec::new();
            for c in full_line {
                row.push(Cell {
                    text: SPACE,
                    color: Some(Char::new_must(c)),
                });
                color += 1;
            }
            rows.push(row);
        }
        Ok(Self { width, color, rows })
    }

    /// Reads a text-only frame from input lines.
    pub fn read_text<R: Read>(lines: &mut io::Lines<BufReader<R>>) -> Result<Self> {
        let mut width: usize = 0;
        let mut rows: Vec<Vec<Cell>> = Vec::new();
        for line in lines {
            let line = normalize_text(line?.as_str());
            if line.is_empty() {
                break;
            }
            let full_line: Vec<char> = line.chars().collect();
            if width != 0 && full_line.len() != width {
                return Err(Error::WidthMismatch);
            }
            width = full_line.len();
            let mut row: Vec<Cell> = Vec::new();
            for c in full_line {
                row.push(Cell {
                    text: Char::new_must(c),
                    color: None,
                });
            }
            rows.push(row);
        }
        Ok(Self {
            width,
            color: 0,
            rows,
        })
    }

    /// Reads a combined (text+color) frame from input lines.
    pub fn read_both<R: Read>(lines: &mut io::Lines<BufReader<R>>) -> Result<Self> {
        let mut width: usize = 0;
        let mut rows: Vec<Vec<Cell>> = Vec::new();
        let mut color: usize = 0;
        for line in lines {
            let line = normalize_text(line?.as_str());
            if line.is_empty() {
                break;
            }
            let full_line: Vec<char> = line.chars().collect();
            if width != 0 && full_line.len() / 2 != width {
                return Err(Error::WidthMismatch);
            }
            width = full_line.len() / 2;
            let mut row: Vec<Cell> = Vec::new();
            let text = &full_line[..full_line.len() / 2];
            let colors = &full_line[full_line.len() / 2..];
            if text.len() != colors.len() {
                return Err(Error::WidthMismatch);
            }
            for i in 0..text.len() {
                row.push(Cell {
                    text: Char::new_must(text[i]),
                    color: Some(Char::new_must(colors[i])),
                });
                color += 1;
            }
            rows.push(row);
        }
        Ok(Self { width, color, rows })
    }
}

impl Frame {
    /// Returns true if the frame contains any color cells.
    pub fn color(&self) -> bool {
        self.color > 0
    }

    /// Returns the width of the frame in cells.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the frame in cells.
    pub fn height(&self) -> usize {
        self.rows.len()
    }

    pub fn remove_color(&mut self, color: Char) {
        for row in &mut self.rows {
            for cell in row {
                if let Some(c) = cell.color {
                    if c == color {
                        cell.color = None;
                    }
                }
            }
        }
    }

    /// Shifts all rows right by `cols`, filling new cells with `fill`.
    pub fn shift_right(&mut self, cols: usize, fill: Cell) {
        let h = self.height();
        let w = self.width();
        if h == 0 || w == 0 {
            return;
        }
        for row in self.rows.iter_mut() {
            if cols <= w {
                row.rotate_right(cols);
            }
            for c in 0..cols.min(w) {
                row[c] = fill;
            }
        }
    }

    /// Shifts all rows left by `cols`, filling vacated cells with `fill`.
    pub fn shift_left(&mut self, cols: usize, fill: Cell) {
        let h = self.height();
        let w = self.width();
        if h == 0 || w == 0 {
            return;
        }
        for row in self.rows.iter_mut() {
            for c in 0..cols.min(w) {
                row[c] = fill;
            }
            if cols <= w {
                row.rotate_left(cols);
            }
        }
    }

    /// Shifts all columns down by `rows`, filling new cells with `fill`.
    pub fn shift_down(&mut self, rows: usize, fill: Cell) {
        let h = self.height();
        if h == 0 {
            return;
        }
        if rows <= h {
            self.rows.rotate_right(rows);
        }
        for r in 0..rows.min(h) {
            self.rows[r] = vec![fill; self.width()];
        }
    }

    /// Shifts all columns up by `rows`, filling vacated cells with `fill`.
    pub fn shift_up(&mut self, rows: usize, fill: Cell) {
        let h = self.height();
        if h == 0 {
            return;
        }
        for r in 0..rows.min(h) {
            self.rows[r] = vec![fill; self.width()];
        }
        if rows <= h {
            self.rows.rotate_left(rows);
        }
    }

    /// Fills a rectangular area defined by column and row iterators with `new` cell.
    pub fn fill_area<C, R>(&mut self, columns: C, rows: R, new: Cell)
    where
        C: IntoIterator<Item = usize>,
        R: IntoIterator<Item = usize>,
    {
        let rows_vec: Vec<usize> = rows.into_iter().collect();
        for column in columns {
            for &row in &rows_vec {
                self.set(column, row, new);
            }
        }
    }

    /// Sets the cell at (column, row) to `new`.
    pub fn set(&mut self, column: usize, row: usize, new: Cell) {
        if column < self.width() && row < self.height() {
            let old = self.rows[row][column];
            self.rows[row][column] = new;
            self.adjust_color(old, new);
        }
    }

    /// Gets the cell at (column, row), returning `default` if out of bounds.
    pub fn get(&self, column: usize, row: usize, default: Cell) -> Cell {
        if column < self.width() && row < self.height() {
            self.rows[row][column]
        } else {
            default
        }
    }

    /// Adjusts frame size to at least `width` and `height`, filling new cells with `fill`.
    pub fn adjust(&mut self, width: usize, height: usize, fill: Cell) {
        self.adjust_width(width, fill);
        self.adjust_height(height, fill);
    }

    /// Adjusts frame width to at least `width`.
    pub fn adjust_width(&mut self, width: usize, fill: Cell) {
        if width > self.width() {
            self.resize_width(width, fill);
        }
    }

    /// Adjusts frame height to at least `height`.
    pub fn adjust_height(&mut self, height: usize, fill: Cell) {
        if height > self.height() {
            self.resize_height(height, fill);
        }
    }

    /// Resizes frame to exact `width` and `height`, filling new cells with `fill`.
    pub fn resize(&mut self, width: usize, height: usize, fill: Cell) {
        self.resize_width(width, fill);
        self.resize_height(height, fill);
    }

    /// Resizes width to exact `width`.
    pub fn resize_width(&mut self, width: usize, fill: Cell) {
        if self.width() != width {
            for row in &mut self.rows {
                row.resize(width, fill);
            }
            self.width = width;
        }
    }

    /// Resizes height to exact `height`.
    pub fn resize_height(&mut self, height: usize, fill: Cell) {
        if self.height() != height {
            let fill_row = vec![fill; self.width()];
            self.rows.resize(height, fill_row);
        }
    }

    /// Checks if the frame contains the given cell.
    pub fn contains(&self, cell: Cell) -> bool {
        for row in &self.rows {
            for c in row {
                if *c == cell {
                    return true;
                }
            }
        }
        false
    }

    /// Checks if the frame contains the given text character.
    pub fn contains_text(&self, ch: Char) -> bool {
        for row in &self.rows {
            for c in row {
                if c.text == ch {
                    return true;
                }
            }
        }
        false
    }

    /// Checks if the frame contains the given color character.
    pub fn contains_color(&self, col: Char) -> bool {
        for row in &self.rows {
            for c in row {
                if c.color == Some(col) {
                    return true;
                }
            }
        }
        false
    }

    /// Clears the frame: sets all text to space and color to default.
    pub fn clean(&mut self) {
        let color = if self.color() { Some(UNDERSCORE) } else { None };
        self.fill(Cell { text: SPACE, color });
    }

    /// Fills the entire frame with the given cell.
    pub fn fill(&mut self, fill: Cell) {
        self.color = if fill.color() {
            self.width() * self.height()
        } else {
            0
        };
        for row in &mut self.rows {
            for cell in row {
                *cell = fill
            }
        }
    }

    /// Fills all text cells with the given character.
    pub fn fill_text(&mut self, fill: Char) {
        for row in &mut self.rows {
            for cell in row {
                cell.text = fill
            }
        }
    }

    /// Fills all color cells with the given character (or None).
    pub fn fill_color(&mut self, fill: Option<Char>) {
        self.color = if fill == None {
            0
        } else {
            self.width() * self.height()
        };
        for row in &mut self.rows {
            for cell in row {
                cell.color = fill
            }
        }
    }

    fn adjust_color(&mut self, old: Cell, new: Cell) {
        match (old.color(), new.color()) {
            (true, true) => {}
            (true, false) => self.color -= 1,
            (false, true) => self.color += 1,
            (false, false) => {}
        }
    }

    /// Prints text to frame.
    pub fn print(&mut self, col: usize, row: usize, line: &str, color: Option<Option<Char>>) {
        let mut col = col;
        for char in line.chars() {
            if let Ok(char) = Char::new(char) {
                let mut cell = self.get(col, row, Cell::default());
                cell.text = char;
                if let Some(color) = color {
                    cell.color = color;
                }
                self.set(col, row, cell);
                col += 1;
            }
        }
    }

    /// Renders the frame as ANSI escape sequences.
    pub fn ansi(&self, palette: &Palette, color: bool) -> String {
        let mut acum = String::new();
        for r in 0..self.height() {
            let row = &self.rows[r];
            if color {
                let mut prev_col: Option<ColorPair> = None;
                for cell in row {
                    let c = cell.to_pair(palette);
                    let ansi = c.to_ansi_rel(&prev_col);
                    if ansi != "" {
                        acum += ansi.as_str();
                    }
                    prev_col = Some(c);
                    acum.push(cell.text.into());
                }
            } else {
                for cell in row {
                    acum.push(cell.text.into());
                }
            }
            if color {
                acum += &ColorPair::default().to_ansi();
            }
            if r + 1 < self.height() {
                acum += "\n";
            }
        }
        acum
    }

    /// Creates a new frame of given dimensions filled with `fill`.
    pub fn new(width: usize, height: usize, fill: Cell) -> Self {
        Self {
            color: if fill.color() { width * height } else { 0 },
            width,
            rows: vec![vec![fill; width]; height],
        }
    }

    /// Formats the frame's text channel.
    pub fn fmt_text(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in &self.rows {
            let mut acum = String::new();
            for cell in row {
                acum.push(cell.text.into());
            }
            writeln!(f, "{}", acum)?;
        }
        Ok(())
    }

    /// Formats the frame's color channel.
    pub fn fmt_colors(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in &self.rows {
            let mut acum = String::new();
            for cell in row {
                acum.push(cell.color.unwrap_or(UNDERSCORE).into());
            }
            writeln!(f, "{}", acum)?;
        }
        Ok(())
    }

    /// Formats both text and color channels concatenated.
    pub fn fmt_both(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in &self.rows {
            let mut acum = String::new();
            for cell in row {
                acum.push(cell.text.into());
            }
            for cell in row {
                acum.push(cell.color.unwrap_or(UNDERSCORE).into());
            }
            writeln!(f, "{}", acum)?;
        }
        Ok(())
    }

    /// Formats the frame with optional color inclusion.
    pub fn fmt_with_colors(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        colors: Option<bool>,
    ) -> std::fmt::Result {
        if let Some(false) = colors {
            self.fmt_text(f)
        } else {
            self.fmt_both(f)
        }
    }
    pub(crate) fn recalc_colors(&mut self) {
        self.color = 0;
        for row in &self.rows {
            for cell in row {
                if cell.color() {
                    self.color += 1;
                }
            }
        }
    }
}

/// Displays the frame in combined text+color format.
impl fmt::Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_both(f)
    }
}

/// A collection of frames forming an animation, with optional pinned channels.
#[derive(Default, Debug, Clone)]
pub struct Frames {
    pub(crate) text_pin: Option<Frame>,
    pub(crate) color_pin: Option<Frame>,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) frames: Vec<Frame>,
}

impl Frames {
    /// Creates a new set of `frames` frames with given dimensions, each filled with `fill`.
    pub fn new(frames: usize, width: usize, height: usize, fill: Cell) -> Self {
        let mut ret = Self {
            text_pin: None,
            color_pin: None,
            width,
            height,
            frames: Vec::with_capacity(frames),
        };
        (0..frames).for_each(|_| {
            ret.frames.push(Frame::new(width, height, fill));
        });
        ret
    }

    pub fn remove_color(&mut self, color: Char) {
        for frame in &mut self.frames {
            frame.remove_color(color);
        }
    }

    /// Sets a cell in a specific frame.
    pub fn set(&mut self, frame: usize, column: usize, row: usize, new: Cell) {
        if frame < self.frames() {
            self.frames[frame].set(column, row, new);
        }
    }

    /// Gets a cell from a specific frame, with default if out of bounds.
    pub fn get(&self, frame: usize, column: usize, row: usize, default: Cell) -> Cell {
        if frame < self.frames() {
            self.frames[frame].get(column, row, default)
        } else {
            default
        }
    }

    /// Ensures that the given frame index exists, extending if necessary.
    pub fn make_sure_frame_exist(&mut self, frame: usize) {
        if frame >= self.frames() {
            if self.frames() == 0 {
                for _ in 0..frame + 1 {
                    self.frames
                        .push(Frame::new(self.width, self.height, Cell::default()));
                }
            } else {
                for _ in 0..(frame + 1 - self.frames()) {
                    self.frames.push(self.frames.last().unwrap().clone());
                }
            }
        }
    }

    /// Duplicates the given frame, inserting the copy before it.
    pub fn dup_frame(&mut self, frame: usize) {
        self.make_sure_frame_exist(frame);
        self.frames.insert(frame, self.frames[frame].clone());
    }

    /// Shifts a specific frame right.
    pub fn shift_right_frame(&mut self, frame: usize, cols: usize, fill: Cell) {
        if frame < self.frames() {
            self.frames[frame].shift_right(cols, fill);
        }
    }

    /// Shifts all frames right.
    pub fn shift_right(&mut self, cols: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.shift_right(cols, fill);
        }
    }

    /// Shifts a specific frame left.
    pub fn shift_left_frame(&mut self, frame: usize, cols: usize, fill: Cell) {
        if frame < self.frames() {
            self.frames[frame].shift_left(cols, fill);
        }
    }

    /// Shifts all frames left.
    pub fn shift_left(&mut self, cols: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.shift_left(cols, fill);
        }
    }

    /// Shifts a specific frame up.
    pub fn shift_up_frame(&mut self, frame: usize, rows: usize, fill: Cell) {
        if frame < self.frames() {
            self.frames[frame].shift_up(rows, fill);
        }
    }

    /// Shifts all frames up.
    pub fn shift_up(&mut self, rows: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.shift_up(rows, fill);
        }
    }

    /// Shifts a specific frame down.
    pub fn shift_down_frame(&mut self, frame: usize, rows: usize, fill: Cell) {
        if frame < self.frames() {
            self.frames[frame].shift_down(rows, fill);
        }
    }

    /// Shifts all frames down.
    pub fn shift_down(&mut self, rows: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.shift_down(rows, fill);
        }
    }

    /// Fills an area in a specific frame.
    pub fn fill_area_frame<C, R>(&mut self, frame: usize, columns: C, rows: R, new: Cell)
    where
        C: IntoIterator<Item = usize>,
        R: IntoIterator<Item = usize>,
    {
        self.frames[frame].fill_area(columns, rows, new);
    }

    /// Fills an area in all frames.
    pub fn fill_area<C, R>(&mut self, columns: C, rows: R, new: Cell)
    where
        C: IntoIterator<Item = usize>,
        R: IntoIterator<Item = usize>,
    {
        let columns_vec: Vec<usize> = columns.into_iter().collect();
        let rows_vec: Vec<usize> = rows.into_iter().collect();
        for frame in self.frames.iter_mut() {
            frame.fill_area(columns_vec.clone(), rows_vec.clone(), new);
        }
    }

    /// Adjusts all frames to at least `width` and `height`.
    pub fn adjust(&mut self, width: usize, height: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.adjust(width, height, fill);
        }
        self.width = width;
        self.height = height;
    }

    /// Adjusts width of all frames.
    pub fn adjust_width(&mut self, width: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.adjust_width(width, fill);
        }
        self.width = width;
    }

    /// Adjusts height of all frames.
    pub fn adjust_height(&mut self, height: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.adjust_height(height, fill);
        }
        self.height = height;
    }

    /// Resizes all frames to exact dimensions.
    pub fn resize(&mut self, width: usize, height: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.resize(width, height, fill);
        }
        self.width = width;
        self.height = height;
    }

    /// Resizes width of all frames.
    pub fn resize_width(&mut self, width: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.resize_width(width, fill);
        }
        self.width = width;
    }

    /// Resizes height of all frames.
    pub fn resize_height(&mut self, height: usize, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.resize_height(height, fill);
        }
        self.height = height;
    }

    /// Clears all frames (text to space, color to underscore if any).
    pub fn clean(&mut self) {
        for frame in self.frames.iter_mut() {
            frame.clean();
        }
    }

    /// Clears a specific frame.
    pub fn clean_frame(&mut self, frame: usize) {
        if frame < self.frames() {
            self.frames[frame].clean();
        }
    }

    /// Fills all frames with the given cell.
    pub fn fill(&mut self, fill: Cell) {
        for frame in self.frames.iter_mut() {
            frame.fill(fill);
        }
    }

    /// Fills a specific frame with the given cell.
    pub fn fill_frame(&mut self, frame: usize, fill: Cell) {
        if frame < self.frames() {
            self.frames[frame].fill(fill);
        }
    }

    /// Fills text of all frames with the given character.
    pub fn fill_text(&mut self, fill: Char) {
        for frame in self.frames.iter_mut() {
            frame.fill_text(fill);
        }
    }

    /// Fills text of a specific frame with the given character.
    pub fn fill_text_frame(&mut self, frame: usize, fill: Char) {
        if frame < self.frames() {
            self.frames[frame].fill_text(fill);
        }
    }

    /// Fills color of all frames with the given character (or None).
    pub fn fill_color(&mut self, fill: Option<Char>) {
        for frame in self.frames.iter_mut() {
            frame.fill_color(fill);
        }
    }

    /// Fills color of a specific frame with the given character (or None).
    pub fn fill_color_frame(&mut self, frame: usize, fill: Option<Char>) {
        if frame < self.frames() {
            self.frames[frame].fill_color(fill);
        }
    }

    /// Prints text to specific frame.
    pub fn print(
        &mut self,
        frame: usize,
        col: usize,
        row: usize,
        line: &str,
        color: Option<Option<Char>>,
    ) {
        if frame < self.frames() {
            self.frames[frame].print(col, row, line, color);
        }
    }

    /// Removes a specific frame.
    pub fn remove_frame(&mut self, frame: usize) {
        if frame < self.frames.len() {
            self.frames.remove(frame);
        }
    }

    /// Checks if any frame contains the given cell.
    pub fn contains(&self, cell: Cell) -> bool {
        for frame in &self.frames {
            if frame.contains(cell) {
                return true;
            }
        }
        false
    }

    /// Checks if any frame contains the given color character.
    pub fn contains_color(&self, name: Char) -> bool {
        for frame in &self.frames {
            if frame.contains_color(name) {
                return true;
            }
        }
        false
    }

    /// Checks if any frame contains the given text character.
    pub fn contains_text(&self, ch: Char) -> bool {
        for frame in &self.frames {
            if frame.contains_color(ch) {
                return true;
            }
        }
        false
    }

    /// Pins the color channel from the given frame to all frames.
    pub fn pin_color(&mut self, frame: usize) -> Result<()> {
        if frame >= self.frames.len() {
            return Ok(());
        }
        self.color_pin = Some(self.frames[frame].clone());
        self.merge()
    }

    /// Pins the text channel from the given frame to all frames.
    pub fn pin_text(&mut self, frame: usize) -> Result<()> {
        if frame >= self.frames.len() {
            return Ok(());
        }
        self.text_pin = Some(self.frames[frame].clone());
        self.merge()
    }

    /// Returns a vector frames converted to text with ANCI escape codes.
    pub fn to_ansi_frames(&self, palette: &Palette, color: bool) -> Vec<String> {
        let mut frames = Vec::new();
        for frame in &self.frames {
            frames.push(frame.ansi(palette, color));
        }
        frames
    }

    /// Returns the number of frames.
    pub fn frames(&self) -> usize {
        self.frames.len()
    }

    /// Returns the width of the frames.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the frames.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns true if any frame contains color.
    pub fn color(&self) -> bool {
        for frame in &self.frames {
            if frame.color() {
                return true;
            }
        }
        false
    }

    /// Returns whether text and color are pinned across frames.
    pub fn pinned(&self) -> (bool, bool) {
        if self.frames.len() < 2 {
            return (false, false);
        }
        let mut text_pinned = true;
        let mut color_pinned = true;
        for c in 0..self.width {
            for r in 0..self.height {
                let mut last_text: Option<Char> = None;
                let mut last_color: Option<Option<Char>> = None;
                for frame in &self.frames {
                    let cell = frame.rows[r][c];
                    if let Some(last_text) = last_text {
                        if last_text != cell.text {
                            text_pinned = false;
                        }
                    }
                    last_text = Some(cell.text);
                    if let Some(last_color) = last_color {
                        if last_color != cell.color {
                            color_pinned = false;
                        }
                    }
                    last_color = Some(cell.color);
                    if !text_pinned && !color_pinned {
                        return (false, false);
                    }
                }
            }
        }
        (text_pinned, color_pinned)
    }

    /// Computes total duration from delays.
    pub fn duration(&self, delays: &Delay) -> usize {
        let mut dur = 0;
        for f in 0..self.frames() {
            dur += delays.get_frame(f);
        }
        dur
    }

    /// Generates an animated SVG from all frames.
    pub fn to_svg_frames(
        &self,
        colors: bool,
        palette: &Palette,
        map: &CSSColorMap,
        font: &Font,
        delays: &Delay,
    ) -> String {
        let delays = delays.to_vec_delays(self.frames());
        let (total_s, key_times, delays) = timing_for_svg(&delays);
        let mut svg = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n".into();
        let width = self.width() * font.width;
        let height = self.height() * font.height;
        svg += format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" role=\"img\">\n",
            width, height, width, height
        )
        .as_str();
        svg += font.to_style().as_str();
        if colors {
            svg += format!(
                "<rect x=\"0\"  y=\"0\"  width=\"{}\" height=\"{}\" fill=\"{}\"/>\n",
                width,
                height,
                map.map(Color::None, false)
            )
            .as_str();
        }
        svg += "\n";
        let (_, color_pinned) = self.pinned();
        if colors && self.color() && color_pinned {
            svg += self.frames[0].to_svg_frame_bg(palette, map, font).as_str();
            for f in 0..self.frames() {
                svg += "<g opacity=\"0\">\n";
                svg += self.frames[f]
                    .to_svg_frame_txt_fg(palette, map, font)
                    .as_str();
                svg += format!(
                "<animate attributeName=\"opacity\" begin=\"0s\" dur=\"{}s\" repeatCount=\"indefinite\" calcMode=\"discrete\" values=\"{}\" keyTimes=\"{}\" />\n",
                total_s, delays[f], key_times
            )
            .as_str();
                svg += "</g>\n\n";
            }
        } else {
            for f in 0..self.frames() {
                svg += "<g opacity=\"0\">\n";
                svg += self.frames[f]
                    .to_svg_frame(colors, palette, map, font)
                    .as_str();
                svg += format!(
                "<animate attributeName=\"opacity\" begin=\"0s\" dur=\"{}s\" repeatCount=\"indefinite\" calcMode=\"discrete\" values=\"{}\" keyTimes=\"{}\" />\n",
                total_s, delays[f], key_times
            )
            .as_str();
                svg += "</g>\n\n";
            }
        }
        svg += "</svg>\n";
        svg
    }
}

impl Frames {
    pub(crate) fn read_text_pin<R: Read>(
        &mut self,
        lines: &mut io::Lines<BufReader<R>>,
    ) -> Result<()> {
        if self.text_pin != None {
            return Err(Error::BlockDup("text-pin".into()));
        }
        let frame = Frame::read_text(lines)?;
        if frame.width() != 0 && frame.height() != 0 {
            self.text_pin = Some(frame)
        }
        Ok(())
    }
    pub(crate) fn read_color_pin<R: Read>(
        &mut self,
        lines: &mut io::Lines<BufReader<R>>,
    ) -> Result<()> {
        if self.color_pin != None {
            return Err(Error::BlockDup("color-pin".into()));
        }
        let frame = Frame::read_color(lines)?;
        if frame.width() != 0 && frame.height() != 0 {
            self.color_pin = Some(frame)
        }
        Ok(())
    }

    pub(crate) fn merge(&mut self) -> Result<()> {
        if let Some(color_pin) = &self.color_pin {
            for i in 0..self.frames.len() {
                self.frames[i] = merge_frames(&self.frames[i], &color_pin)?;
            }
        }
        if let Some(text_pin) = &self.text_pin {
            for i in 0..self.frames.len() {
                self.frames[i] = merge_frames(&text_pin, &self.frames[i])?;
            }
        }
        self.color_pin = None;
        self.text_pin = None;
        Ok(())
    }

    pub(crate) fn check_frame(&mut self, frame: &Frame) -> Result<()> {
        if self.width != 0 && self.width != frame.width() {
            return Err(Error::WidthMismatch);
        }
        if self.height != 0 && self.height != frame.height() {
            return Err(Error::HeightMismatch);
        }
        self.width = frame.width();
        self.height = frame.height();
        Ok(())
    }

    pub(crate) fn read_body<R: Read>(
        &mut self,
        lines: &mut io::Lines<BufReader<R>>,
        header: &Header,
    ) -> Result<()> {
        if !header.get_colors() {
            self.read_body_text(lines)
        } else if self.color_pin != None {
            self.read_body_text(lines)
        } else if self.text_pin != None {
            self.read_body_color(lines)
        } else {
            self.read_body_both(lines)
        }
    }

    pub(crate) fn read_body_both<R: Read>(
        &mut self,
        lines: &mut io::Lines<BufReader<R>>,
    ) -> Result<()> {
        loop {
            let frame = Frame::read_both(lines)?;
            if frame.width() == 0 || frame.height() == 0 {
                break;
            }
            self.check_frame(&frame)?;
            self.frames.push(frame);
        }
        self.merge()
    }

    pub(crate) fn read_body_text<R: Read>(
        &mut self,
        lines: &mut io::Lines<BufReader<R>>,
    ) -> Result<()> {
        loop {
            let frame = Frame::read_text(lines)?;
            if frame.width() == 0 || frame.height() == 0 {
                break;
            }
            self.check_frame(&frame)?;
            self.frames.push(frame);
        }
        self.merge()
    }

    pub(crate) fn read_body_color<R: Read>(
        &mut self,
        lines: &mut io::Lines<BufReader<R>>,
    ) -> Result<()> {
        loop {
            let frame = Frame::read_color(lines)?;
            if frame.width() == 0 || frame.height() == 0 {
                break;
            }
            self.check_frame(&frame)?;
            self.frames.push(frame);
        }
        self.merge()
    }

    pub(crate) fn read_legacy<R: Read>(
        info: LegacyHeaderInfo,
        lines: &mut io::Lines<BufReader<R>>,
    ) -> Result<Self> {
        let mut frames = Self {
            width: info.width,
            height: info.height,
            text_pin: None,
            color_pin: None,
            frames: Vec::new(),
        };
        let mut frame: Frame = Frame {
            color: 0,
            width: info.width,
            rows: Vec::new(),
        };
        let mut row: Vec<Cell> = Vec::new();
        let mut fg_len: usize = 0;
        let mut bg_len: usize = 0;
        let mut mode = LegacyScanMode::Text;

        for line in lines {
            let mut comment = false;
            let line = line?;
            let line = match line.split_once("\t") {
                Some((a, _)) => {
                    if a.is_empty() {
                        continue;
                    }
                    a
                }
                None => &line,
            };
            let line = normalize_text(line);
            if line.is_empty() {
                continue;
            }
            for c in line.chars() {
                if comment {
                    continue;
                } else if c == '\t' {
                    comment = true;
                    continue;
                }

                match mode {
                    LegacyScanMode::Text => {
                        row.push(Cell {
                            text: Char::new_must(c),
                            color: None,
                        });
                        if row.len() == info.width {
                            mode = mode.next(info.colors);
                            if info.colors == LegacyColorMode::None {
                                frame.rows.push(row);
                                row = Vec::new();
                            }
                        }
                    }
                    LegacyScanMode::Fg => {
                        row[fg_len].color = Some(Char::new_must(trans_color(c)));
                        fg_len += 1;
                        frame.color += 1;
                        if fg_len == info.width {
                            mode = mode.next(info.colors);
                            fg_len = 0;
                            if info.colors == LegacyColorMode::FgOnly {
                                frame.rows.push(row);
                                row = Vec::new();
                            }
                        }
                    }
                    LegacyScanMode::Bg => {
                        bg_len += 1;
                        if bg_len == info.width {
                            mode = mode.next(info.colors);
                            bg_len = 0;
                            if info.colors == LegacyColorMode::BgOnly
                                || info.colors == LegacyColorMode::FgAndBg
                            {
                                frame.rows.push(row);
                                row = Vec::new();
                            }
                        }
                    }
                }
                if frame.rows.len() == info.height {
                    frames.frames.push(frame);
                    frame = Frame {
                        color: 0,
                        width: info.width,
                        rows: Vec::new(),
                    };
                }
            }
        }

        Ok(frames)
    }

    /// Formats the body with only text channels.
    pub fn fmt_body_text(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "@body")?;
        for frame in &self.frames {
            frame.fmt_text(f)?;
            writeln!(f, "")?;
        }
        Ok(())
    }

    /// Formats the body with only color channels.
    pub fn fmt_body_colors(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "@body")?;
        for frame in &self.frames {
            frame.fmt_colors(f)?;
            writeln!(f, "")?;
        }
        Ok(())
    }

    /// Formats the body with combined text+color channels.
    pub fn fmt_body_both(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "@body")?;
        for frame in &self.frames {
            frame.fmt_both(f)?;
            writeln!(f, "")?;
        }
        Ok(())
    }

    /// Formats the pinned text channel.
    pub fn fmt_pinned_text(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "@text-pin")?;
        if let Some(frame) = &self.frames.first() {
            frame.fmt_text(f)?;
            writeln!(f, "")?;
        }
        Ok(())
    }

    /// Formats the pinned color channel.
    pub fn fmt_pinned_colors(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "@color-pin")?;
        if let Some(frame) = &self.frames.first() {
            frame.fmt_colors(f)?;
            writeln!(f, "")?;
        }
        Ok(())
    }

    /// Formats with optional color inclusion, using pins if possible.
    pub fn fmt_with_color(&self, f: &mut std::fmt::Formatter<'_>, color: bool) -> std::fmt::Result {
        if color {
            let (text_pinned, colors_pinned) = self.pinned();
            if colors_pinned {
                self.fmt_pinned_colors(f)?;
                self.fmt_body_text(f)
            } else if text_pinned {
                self.fmt_pinned_text(f)?;
                self.fmt_body_colors(f)
            } else {
                self.fmt_body_both(f)
            }
        } else {
            self.fmt_body_text(f)
        }
    }
}

/// Displays the frames using optimal format (with pins if possible).
impl fmt::Display for Frames {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_with_color(f, self.color())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum LegacyScanMode {
    Text,
    Fg,
    Bg,
}

impl LegacyScanMode {
    fn next(self, cm: LegacyColorMode) -> Self {
        match (self, cm) {
            (Self::Text, LegacyColorMode::None) => Self::Text,
            (Self::Text, LegacyColorMode::FgOnly) => Self::Fg,
            (Self::Text, LegacyColorMode::BgOnly) => Self::Bg,
            (Self::Text, LegacyColorMode::FgAndBg) => Self::Fg,
            (Self::Fg, LegacyColorMode::FgAndBg) => Self::Bg,
            (Self::Fg, _) => Self::Text,
            (Self::Bg, _) => Self::Text,
        }
    }
}

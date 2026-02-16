use core::fmt;
use io::Write;
use ordermap::OrderMap;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Read};
use std::path::Path;
use std::str::FromStr;
use std::convert::TryFrom;

use crate::CSSColorMap;
use crate::chars::Char;
use crate::content::Cell;
use crate::error::{Error, Result};
use crate::font::Font;
use crate::helpers::json_quote;
use crate::{ColorPair, Comments, Palette, content::Frame, delay::Delay, header::ExtraHeaderKey};
use crate::{chars::normalize_text, content::Frames, header::Header};

#[derive(Debug, Clone)]
pub struct Art {
    pub(crate) header: Header,
    pub(crate) frames: Frames,
    pub(crate) attached: Option<String>,
    pub(crate) extra: Vec<ExtraBlock>,
}

impl Art {
    pub fn new(frames: usize, width: usize, height: usize, fill: Cell) -> Self {
        Self {
            header: Header::default(),
            frames: Frames::new(frames, width, height, fill),
            attached: None,
            extra: Vec::new(),
        }
    }
    pub fn color(&self) -> bool {
        if let Some(colors) = self.header.colors {
            return colors;
        } else {
            self.frames.color() || self.header.palette.len() > 0
        }
    }
    pub fn frames(&self) -> usize {
        self.frames.frames()
    }
    pub fn frame(&self, frame: usize) -> Option<Frame> {
        if frame < self.frames() {
            Some(self.frames.frames[frame].clone())
        } else {
            None
        }
    }
    pub fn width(&self) -> usize {
        self.frames.width()
    }
    pub fn height(&self) -> usize {
        self.frames.height()
    }
}

// Frames passthrough
impl Art {
    pub fn set(&mut self, frame: usize, column: usize, row: usize, new: Cell) {
        self.frames.set(frame, column, row, new);
    }
    pub fn get(&self, frame: usize, column: usize, row: usize, default: Cell) -> Cell {
        self.frames.get(frame, column, row, default)
    }
    pub fn pin_color(&mut self, frame: usize) -> Result<()> {
        self.frames.pin_color(frame)
    }
    pub fn pin_text(&mut self, frame: usize) -> Result<()> {
        self.frames.pin_text(frame)
    }
    // (text_pinned, color_pinned)
    pub fn pinned(&self) -> (bool, bool) {
        self.frames.pinned()
    }
    pub fn shift_right_frame(&mut self, frame: usize, cols: usize, fill: Cell) {
        self.frames.shift_right_frame(frame, cols, fill);
    }
    pub fn shift_right(&mut self, cols: usize, fill: Cell) {
        self.frames.shift_right(cols, fill);
    }
    pub fn shift_left_frame(&mut self, frame: usize, cols: usize, fill: Cell) {
        self.frames.shift_left_frame(frame, cols, fill);
    }
    pub fn shift_left(&mut self, cols: usize, fill: Cell) {
        self.frames.shift_left(cols, fill);
    }
    pub fn shift_up_frame(&mut self, frame: usize, rows: usize, fill: Cell) {
        self.frames.shift_up_frame(frame, rows, fill);
    }
    pub fn shift_up(&mut self, rows: usize, fill: Cell) {
        self.frames.shift_up(rows, fill);
    }
    pub fn shift_down_frame(&mut self, frame: usize, rows: usize, fill: Cell) {
        self.frames.shift_down_frame(frame, rows, fill);
    }
    pub fn shift_down(&mut self, rows: usize, fill: Cell) {
        self.frames.shift_down(rows, fill);
    }
    pub fn fill_area_frame<C, R>(&mut self, frame: usize, columns: C, rows: R, new: Cell)
    where
        C: IntoIterator<Item = usize>,
        R: IntoIterator<Item = usize>,
    {
        self.frames.fill_area_frame(frame, columns, rows, new);
    }
    pub fn fill_area<C, R>(&mut self, columns: C, rows: R, new: Cell)
    where
        C: IntoIterator<Item = usize>,
        R: IntoIterator<Item = usize>,
    {
        self.frames.fill_area(columns, rows, new);
    }
    pub fn adjust(&mut self, width: usize, height: usize, fill: Cell) {
        self.frames.adjust(width, height, fill);
    }
    pub fn adjust_width(&mut self, width: usize, fill: Cell) {
        self.frames.adjust_width(width, fill);
    }
    pub fn adjust_height(&mut self, height: usize, fill: Cell) {
        self.frames.adjust_height(height, fill);
    }
    pub fn resize(&mut self, width: usize, height: usize, fill: Cell) {
        self.frames.resize(width, height, fill);
    }
    pub fn resize_width(&mut self, width: usize, fill: Cell) {
        self.frames.resize_width(width, fill);
    }
    pub fn resize_height(&mut self, height: usize, fill: Cell) {
        self.frames.resize_height(height, fill);
    }
    pub fn clean(&mut self) {
        self.frames.clean();
    }
    pub fn clean_frame(&mut self, frame: usize) {
        self.frames.clean_frame(frame);
    }
    pub fn fill(&mut self, fill: Cell) {
        self.frames.fill(fill);
    }
    pub fn fill_text(&mut self, fill: Char) {
        self.frames.fill_text(fill);
    }
    pub fn fill_text_frame(&mut self, frame: usize, fill: Char) {
        self.frames.fill_text_frame(frame, fill);
    }
    pub fn fill_color(&mut self, fill: Option<Char>) {
        self.frames.fill_color(fill);
    }
    pub fn fill_color_frame(&mut self, frame: usize, fill: Option<Char>) {
        self.frames.fill_color_frame(frame, fill);
    }
}

// Header passthrough
impl Art {
    pub fn title_line(&self) -> String {
        self.header.title_line()
    }
    pub fn authors_line(&self) -> String {
        self.header.authors_line()
    }
    pub fn remove_all_tags(&mut self) {
        self.header.remove_all_tags();
    }
    pub fn remove_tag(&mut self, tag: &str) {
        self.header.remove_tag(tag);
    }
    pub fn add_tag(&mut self, tag: &str) {
        self.header.add_tag(tag);
    }
    pub fn tags(&self) -> HashSet<String> {
        self.header.tags()
    }
    pub fn contains_tag(&self, tag: &str) -> bool {
        self.header.contains_tag(tag)
    }
    pub fn strip_comments(&mut self) {
        self.header.strip_comments();
    }
    pub fn get_title_key(&self) -> Option<String> {
        self.header.title.clone()
    }
    pub fn set_title_key(&mut self, title: Option<String>) {
        self.header.title = title
    }
    pub fn get_colors_key(&self) -> Option<bool> {
        self.header.colors
    }
    pub fn set_colors_key(&mut self, colors: Option<bool>) {
        self.header.colors = colors;
    }
    pub fn get_color_map(&self, name: Char) -> ColorPair {
        self.header.get_color_map(name)
    }
    pub fn set_color_map(&mut self, name: Char, col: ColorPair) {
        self.header.set_color_map(name, col);
    }
    pub fn remove_color_map(&mut self, name: Char) {
        self.header.remove_color_map(name);
    }
    pub fn get_authors_key(&self) -> Vec<String> {
        self.header.authors.keys().map(|k| k.clone()).collect()
    }
    pub fn set_authors_key(&mut self, authors: &[String]) {
        let mut authors_map = OrderMap::<String, Comments>::new();
        for author in authors {
            authors_map.insert(author.into(), Vec::new());
        }
        self.header.authors = authors_map;
    }
    pub fn add_author(&mut self, author: &str) {
        if !self.header.authors.contains_key(author) {
            self.header.authors.insert(author.into(), Vec::new());
        }
    }
    pub fn get_orig_authors_key(&self) -> Vec<String> {
        self.header.orig_authors.keys().map(|k| k.clone()).collect()
    }
    pub fn set_orig_authors_key(&mut self, authors: &[String]) {
        let mut authors_map = OrderMap::<String, Comments>::new();
        for author in authors {
            authors_map.insert(author.into(), Vec::new());
        }
        self.header.orig_authors = authors_map;
    }
    pub fn add_orig_author(&mut self, author: &str) {
        if !self.header.orig_authors.contains_key(author) {
            self.header.orig_authors.insert(author.into(), Vec::new());
        }
    }
    pub fn remove_author(&mut self, author: &str) {
        self.header.authors.remove(author);
        self.header.orig_authors.remove(author);
    }
    // (is orig, is author)
    pub fn check_author(&self, author: &str) -> (bool, bool) {
        (
            self.header.orig_authors.contains_key(author),
            self.header.authors.contains_key(author),
        )
    }
    pub fn get_src_key(&self) -> Option<String> {
        self.header.src.clone()
    }
    pub fn set_src_key(&mut self, src: Option<String>) {
        self.header.src = src;
    }
    pub fn get_editor_key(&self) -> Option<String> {
        self.header.editor.clone()
    }
    pub fn set_editor_key(&mut self, editor: Option<String>) {
        self.header.editor = editor;
    }
    pub fn get_license_key(&self) -> Option<String> {
        self.header.license.clone()
    }
    pub fn set_license_key(&mut self, license: Option<String>) {
        self.header.license = license;
    }
    pub fn get_loop_key(&self) -> bool {
        if let Some(flag) = self.header.loop_flag {
            flag
        } else {
            true
        }
    }
    pub fn set_loop_key(&mut self, flag: bool) {
        if !flag || self.header.loop_comments.len() > 0 {
            self.header.loop_flag = Some(flag)
        } else {
            self.header.loop_flag = None
        }
    }
    pub fn get_preview_key(&self) -> Option<usize> {
        if let Some(preview) = self.header.preview {
            if preview < self.frames() {
                Some(preview)
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn set_preview_key(&mut self, preview: Option<usize>) {
        if let Some(preview) = preview {
            if preview < self.frames() {
                self.header.preview = Some(preview)
            }
        } else {
            self.header.preview = None;
        }
    }

    pub fn get_global_delay(&self) -> usize {
        if let Some(delay) = &self.header.delay {
            delay.get_global()
        } else {
            50
        }
    }
    pub fn get_frame_delay(&self, frame: usize) -> usize {
        if let Some(delay) = &self.header.delay {
            delay.get_frame(frame)
        } else {
            50
        }
    }
    pub fn set_global_delay(&mut self, global: usize) {
        if let Some(d) = &mut self.header.delay {
            d.set_global(global);
        } else {
            if global == 50 {
                return;
            }
            self.header.delay = Some(Delay {
                global,
                per_frame: HashMap::new(),
            })
        }
    }
    pub fn set_frame_delay(&mut self, frame: usize, delay: usize) {
        if let Some(d) = &mut self.header.delay {
            d.set_frame(frame, delay);
        } else {
            if frame >= self.frames() {
                return;
            }
            let mut map = HashMap::new();
            map.insert(frame, delay);
            self.header.delay = Some(Delay {
                global: 50,
                per_frame: map,
            })
        }
    }
    pub fn reset_delays(&mut self, delay: Option<Delay>) {
        if delay == None {
            self.header.delay_comments = Vec::new();
        }
        self.header.delay = delay;
    }
    pub fn get_extra_keys(&self) -> Vec<ExtraHeaderKey> {
        self.header.extra_keys.clone()
    }
    pub fn set_extra_keys(&mut self, extra: Vec<ExtraHeaderKey>) {
        self.header.extra_keys = extra
    }
    pub fn contains(&self, cell: Cell) -> bool {
        self.frames.contains(cell)
    }
    pub fn contains_text(&self, ch: Char) -> bool {
        self.frames.contains_text(ch)
    }
    pub fn contains_color(&self, name: Char) -> bool {
        self.header.contains_color(name) || self.frames.contains_color(name)
    }
    pub fn free_color_name(&self) -> Char {
        // Try some well known chars
        for name in
            "ghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-+,.~?!@#$%^&*`<>()[]{}\"'\\|/:;"
                .chars()
        {
            if let Ok(name) = Char::new(name) {
                if !self.contains_color(name) {
                    return name;
                }
            }
        }
        for name in "abcdef№¢£¥€°±÷¶§µ•…¬≈≠≤≥∞∆∂∑∏∫√■□●○▲△▼▽▶▷◀◁◆◇★☆❤♡♠♤♣♧♦♢←↑→↓↔↕↖↗↘↙⇐⇑⇒⇓⇔⇕↜↝αβγδζεηΘλξΞπστφωΩбгдёилпуфцчшъыэюяᚠᚢᚤᚣᚥᚦᚧᚨᚩᚫᚬᚭᚮᚯᚱᚳᚴᚸᚹᚻᚼᚽᚾᛃᛄᛇᛈᛉᛊᛋᛔᛗᛘᛗᛙᛜᛝᛟᛢᛣᛥᛦᛪ".chars() {
            if let Ok(name) = Char::new(name) {
                if !self.contains_color(name) {
                    return name;
                }
            }
        }
        // Try all existed unicode space
        for code in 0..u32::MAX {
            if let Some(name) = char::from_u32(code) {
                if let Ok(name) = Char::new(name) {
                    if !self.contains_color(name) {
                        return name;
                    }
                }
            }
        }
        panic!("literally all billons possible chars are used in current palette");
    }

    pub fn set_palette(&mut self, palette: Palette) {
        self.header.palette = palette
    }
    pub fn remove_palette(&mut self) {
        self.header.palette = Palette::default();
    }
    pub fn search_color_map(&self, col: ColorPair) -> Option<Char> {
        self.header.search_color_map(col)
    }
    pub fn search_or_create_color_map(&mut self, col: ColorPair) -> Char {
        if let Some(name) = self.search_color_map(col) {
            name
        } else {
            let name = self.free_color_name();
            self.set_color_map(name, col);
            name
        }
    }

    pub fn remove_frame(&mut self, frame: usize) {
        self.frames.remove_frame(frame);
    }
    pub fn make_sure_frame_exist(&mut self, frame: usize) {
        self.frames.make_sure_frame_exist(frame);
    }
    pub fn dup_frame(&mut self, frame: usize) {
        self.frames.dup_frame(frame);
    }
}

// Conversions
impl Art {
    // Total duration in secs
    pub fn duration(&self) -> f64 {
        let mut dur: usize = 0;
        for f in 0..self.frames() {
            dur += self.get_frame_delay(f);
        }
        dur as f64 / 1000.0
    }
    pub fn to_asciicast2(&self) -> String {
        let dur = self.duration();
        let mut cast = match self.header.title {
            Some(_) => format!(
                "{{\"version\": 2, \"width\": {}, \"height\": {}, \"duration\": {}, \"title\": {} }}\n",
                self.width(),
                self.height(),
                dur,
                json_quote(&self.title_line())
            ),
            None => format!(
                "{{\"version\": 2, \"width\": {}, \"height\": {}, \"duration\": {} }}\n",
                self.width(),
                self.height(),
                dur
            ),
        };
        cast += format!("[0, \"o\", {}]\n", json_quote("\x1b[?25l")).as_str();
        let mut cum_time: usize = 0;
        let color = self.color();
        let h = self.height();
        let h = if h > 1 { h - 1 } else { h };
        for f in 0..self.frames() {
            let frame = &self.frames.frames[f];
            let time = (cum_time as f64) / 1000.0;
            let ansi = frame.ansi(&self.header.palette, color);
            let ansi = ansi.replace("\n", "\n\r") + format!("\r\x1b[{}A", h).as_str();
            cast += format!("[{}, \"o\", {}]\n", time, json_quote(&ansi)).as_str();
            cum_time += self.get_frame_delay(f)
        }
        cast += format!("[{}, \"o\", {}]\n", dur, json_quote(&"\n".repeat(h))).as_str();
        cast += format!("[{}, \"o\", {}]\n", dur, json_quote("\x1b[?25h")).as_str();
        cast
    }
    pub fn to_svg_frames(&self, map: &CSSColorMap, font: &Font) -> String {
        let delay = self.header.delay.clone().unwrap_or(Delay::default());
        self.frames
            .to_svg_frames(self.color(), &self.header.palette, map, font, &delay)
    }
    pub fn to_ansi_frames(&self) -> Vec<String> {
        self.frames
            .to_ansi_frames(&self.header.palette, self.color())
    }
    pub fn to_ansi_string(&self) -> String {
        format!(
            "{}{}\n",
            self.to_ansi_frames().join("\n"),
            ColorPair::default().to_ansi()
        )
    }
    pub fn to_ansi_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        writeln!(
            file,
            "{}{}",
            self.to_ansi_frames().join("\n"),
            ColorPair::default().to_ansi()
        )
    }
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        write!(file, "{}", self)
    }
    pub fn to_components(self) -> (Header, Frames, Option<String>, Vec<ExtraBlock>) {
        (self.header, self.frames, self.attached, self.extra)
    }
    pub fn from_components(
        header: Header,
        frames: Frames,
        attached: Option<String>,
        extra: Vec<ExtraBlock>,
    ) -> Result<Self> {
        Ok(Self {
            header,
            frames,
            attached,
            extra,
        })
    }
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_reader(File::open(path)?)
    }
    pub fn from_reader<R: Read>(r: R) -> Result<Self> {
        let mut lines = BufReader::new(r).lines();
        Self::from_lines(&mut lines)
    }
    pub fn from_lines<R: Read>(lines: &mut io::Lines<BufReader<R>>) -> Result<Self> {
        let header = Header::read(lines)?;
        let mut frames = Frames {
            text_pin: None,
            color_pin: None,
            width: 0,
            height: 0,
            frames: Vec::new(),
        };
        let mut attached: Option<String> = None;
        let mut extra: Vec<ExtraBlock> = Vec::new();
        if let Some(legacy) = header.legacy {
            frames = Frames::read_legacy(legacy, lines)?;
        } else {
            loop {
                let title = next_block(lines);
                match title {
                    Ok(Some(blk)) => match blk.as_str() {
                        "attach" => {
                            if let Some(line) = lines.next() {
                                attached = Some(line?);
                            }
                        }
                        "text-pin" => {
                            frames.read_text_pin(lines)?;
                        }
                        "color-pin" => {
                            frames.read_color_pin(lines)?;
                        }
                        "body" => {
                            frames.read_body(lines, &header)?;
                        }
                        title => {
                            extra.push(ExtraBlock::read(title, lines)?);
                        }
                    },
                    Ok(None) => {
                        break;
                    }
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
        }
        frames.merge()?;
        Self::from_components(header, frames, attached, extra)
    }
}

impl FromStr for Art {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::from_reader(Cursor::new(s.as_bytes()))
    }
}

impl TryFrom<&str> for Art {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self> {
        Self::from_reader(Cursor::new(value.as_bytes()))
    }
}

impl TryFrom<&[u8]> for Art {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        Self::from_reader(Cursor::new(value))
    }
}

impl Into<String> for Art {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl Into<String> for &Art {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl Into<Vec<String>> for Art {
    fn into(self) -> Vec<String> {
        self.to_ansi_frames()
    }
}

impl Into<Vec<String>> for &Art {
    fn into(self) -> Vec<String> {
        self.to_ansi_frames()
    }
}

impl fmt::Display for Art {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.header.fmt_with_colors(f, self.color())?;
        if let Some(attached) = &self.attached {
            if attached.len() > 0 {
                writeln!(f, "@attach\n{}\n", attached)?;
            }
        }
        for extra in &self.extra {
            extra.fmt(f)?;
        }
        self.frames.fmt_with_color(f, self.color())?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ExtraBlock {
    pub title: String,
    pub content: String,
}

impl ExtraBlock {
    pub(crate) fn read<R: Read>(title: &str, lines: &mut io::Lines<BufReader<R>>) -> Result<Self> {
        let mut block = ExtraBlock {
            title: title.into(),
            content: "".into(),
        };
        for line in lines {
            let line = normalize_text(line?.as_str());
            if line.is_empty() {
                break;
            }
            block.content += &line;
            block.content += "\n";
        }
        Ok(block)
    }
}

impl fmt::Display for ExtraBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "@{}", self.title)?;
        writeln!(f, "{}", self.content)?;
        Ok(())
    }
}

pub(crate) fn next_block<R: Read>(lines: &mut io::Lines<BufReader<R>>) -> Result<Option<String>> {
    for line in lines {
        let line = normalize_text(line?.as_str());
        if line.is_empty() {
            continue;
        }
        return match line.strip_prefix("@") {
            Some(name) => Ok(Some(name.into())),
            None => Err(Error::BlockExpected(line)),
        };
    }
    Ok(None)
}

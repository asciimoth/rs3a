use core::fmt;
use io::Write;
use ordermap::OrderMap;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::default;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Read};
use std::path::Path;
use std::str::FromStr;

use crate::chars::{Char, UNDERSCORE};
use crate::colors::apply_sgr;
use crate::content::Cell;
use crate::error::{Error, Result};
use crate::font::Font;
use crate::helpers::json_quote;
use crate::ttyrec::{TtyrecFrame, TtyrecReader};
use crate::{chars::normalize_text, content::Frames, header::Header};
use crate::{content::Frame, delay::Delay, header::ExtraHeaderKey, ColorPair, Comments, Palette};
use crate::{CSSColorMap, Color, Color4};

/// Represents a complete 3a ASCII art animation, including header, frames,
/// attached content, and extra blocks.
#[derive(Debug, Clone)]
pub struct Art {
    pub(crate) header: Header,
    pub(crate) frames: Frames,
    pub(crate) attached: Option<String>,
    pub(crate) extra: Vec<ExtraBlock>,
}

impl Art {
    /// Creates a new Art with the specified number of frames, width, height,
    /// and fill cell.
    pub fn new(frames: usize, width: usize, height: usize, fill: Cell) -> Self {
        Self {
            header: Header::default(),
            frames: Frames::new(frames, width, height, fill),
            attached: None,
            extra: Vec::new(),
        }
    }

    /// Returns whether the art uses colors (either explicitly set in header or
    /// detected in frames).
    pub fn color(&self) -> bool {
        if let Some(colors) = self.header.colors {
            return colors;
        } else {
            self.frames.color() || self.header.palette.len() > 0
        }
    }

    /// Returns the number of frames.
    pub fn frames(&self) -> usize {
        self.frames.frames()
    }

    /// Returns a clone of the frame at the given index, if it exists.
    pub fn frame(&self, frame: usize) -> Option<Frame> {
        if frame < self.frames() {
            Some(self.frames.frames[frame].clone())
        } else {
            None
        }
    }

    /// Returns the width of the art in columns.
    pub fn width(&self) -> usize {
        self.frames.width()
    }

    /// Returns the height of the art in rows.
    pub fn height(&self) -> usize {
        self.frames.height()
    }
}

// Frames passthrough
impl Art {
    /// Sets the cell at the specified frame, column, and row.
    pub fn set(&mut self, frame: usize, column: usize, row: usize, new: Cell) {
        self.frames.set(frame, column, row, new);
    }

    /// Gets the cell at the specified frame, column, and row,
    /// falling back to a default if out of bounds.
    pub fn get(&self, frame: usize, column: usize, row: usize, default: Cell) -> Cell {
        self.frames.get(frame, column, row, default)
    }

    /// Remove all frames out of inclusive subrange
    pub fn slice(&mut self, from: usize, to: usize) {
        self.frames.slice(from, to);
    }

    /// Swap two frames
    pub fn swap(&mut self, a: usize, b: usize) {
        self.frames.swap(a, b);
    }

    /// Reverse frames
    pub fn reverse(&mut self) {
        self.frames.reverse();
    }

    /// Deduplicate frames
    pub fn dedup(&mut self) {
        self.frames.dedup();
    }

    /// Rotate frames foth
    pub fn rot_forth(&mut self, k: usize) {
        self.frames.rot_forth(k);
    }

    /// Rotate frames back
    pub fn rot_back(&mut self, k: usize) {
        self.frames.rot_back(k);
    }

    /// Crop art
    pub fn crop(&mut self, r_from: usize, r_to: usize, c_from: usize, c_to: usize) {
        self.frames.crop(r_from, r_to, c_from, c_to);
    }

    /// Pins the color channel from the given frame to all frames.
    pub fn pin_color(&mut self, frame: usize) -> Result<()> {
        self.frames.pin_color(frame)
    }

    /// Pins the text channel from the given frame to all frames.
    pub fn pin_text(&mut self, frame: usize) -> Result<()> {
        self.frames.pin_text(frame)
    }

    /// Returns whether text and color are pinned across frames.
    pub fn pinned(&self) -> (bool, bool) {
        self.frames.pinned()
    }

    /// Shifts a specific frame right.
    pub fn shift_right_frame(&mut self, frame: usize, cols: usize, fill: Cell) {
        self.frames.shift_right_frame(frame, cols, fill);
    }

    /// Shifts all frames right.
    pub fn shift_right(&mut self, cols: usize, fill: Cell) {
        self.frames.shift_right(cols, fill);
    }

    /// Shifts a specific frame left.
    pub fn shift_left_frame(&mut self, frame: usize, cols: usize, fill: Cell) {
        self.frames.shift_left_frame(frame, cols, fill);
    }

    /// Shifts all frames left.
    pub fn shift_left(&mut self, cols: usize, fill: Cell) {
        self.frames.shift_left(cols, fill);
    }

    /// Shifts a specific frame up.
    pub fn shift_up_frame(&mut self, frame: usize, rows: usize, fill: Cell) {
        self.frames.shift_up_frame(frame, rows, fill);
    }

    /// Shifts all frames up.
    pub fn shift_up(&mut self, rows: usize, fill: Cell) {
        self.frames.shift_up(rows, fill);
    }

    /// Shifts a specific frame down.
    pub fn shift_down_frame(&mut self, frame: usize, rows: usize, fill: Cell) {
        self.frames.shift_down_frame(frame, rows, fill);
    }

    /// Shifts all frames down.
    pub fn shift_down(&mut self, rows: usize, fill: Cell) {
        self.frames.shift_down(rows, fill);
    }

    /// Fills an area in a specific frame.
    pub fn fill_area_frame<C, R>(&mut self, frame: usize, columns: C, rows: R, new: Cell)
    where
        C: IntoIterator<Item = usize>,
        R: IntoIterator<Item = usize>,
    {
        self.frames.fill_area_frame(frame, columns, rows, new);
    }

    /// Fills an area in all frames.
    pub fn fill_area<C, R>(&mut self, columns: C, rows: R, new: Cell)
    where
        C: IntoIterator<Item = usize>,
        R: IntoIterator<Item = usize>,
    {
        self.frames.fill_area(columns, rows, new);
    }

    /// Adjusts all frames to at least `width` and `height`.
    pub fn adjust(&mut self, width: usize, height: usize, fill: Cell) {
        self.frames.adjust(width, height, fill);
    }

    /// Adjusts width of all frames.
    pub fn adjust_width(&mut self, width: usize, fill: Cell) {
        self.frames.adjust_width(width, fill);
    }

    /// Adjusts height of all frames.
    pub fn adjust_height(&mut self, height: usize, fill: Cell) {
        self.frames.adjust_height(height, fill);
    }

    /// Resizes all frames to exact dimensions.
    pub fn resize(&mut self, width: usize, height: usize, fill: Cell) {
        self.frames.resize(width, height, fill);
    }

    /// Resizes width of all frames.
    pub fn resize_width(&mut self, width: usize, fill: Cell) {
        self.frames.resize_width(width, fill);
    }

    /// Resizes height of all frames.
    pub fn resize_height(&mut self, height: usize, fill: Cell) {
        self.frames.resize_height(height, fill);
    }

    /// Clears all frames (text to space, color to underscore if any).
    pub fn clean(&mut self) {
        self.frames.clean();
    }

    /// Clears a specific frame.
    pub fn clean_frame(&mut self, frame: usize) {
        self.frames.clean_frame(frame);
    }

    /// Fills all frames with the given cell.
    pub fn fill(&mut self, fill: Cell) {
        self.frames.fill(fill);
    }

    /// Fills a specific frame with the given cell.
    pub fn fill_frame(&mut self, frame: usize, fill: Cell) {
        self.frames.fill_frame(frame, fill);
    }

    /// Fills text of all frames with the given character.
    pub fn fill_text(&mut self, fill: Char) {
        self.frames.fill_text(fill);
    }

    /// Fills text of a specific frame with the given character.
    pub fn fill_text_frame(&mut self, frame: usize, fill: Char) {
        self.frames.fill_text_frame(frame, fill);
    }

    /// Fills color of all frames with the given character (or None).
    pub fn fill_color(&mut self, fill: Option<Char>) {
        self.frames.fill_color(fill);
    }

    /// Fills color of a specific frame with the given character (or None).
    pub fn fill_color_frame(&mut self, frame: usize, fill: Option<Char>) {
        self.frames.fill_color_frame(frame, fill);
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
        self.frames.print(frame, col, row, line, color);
    }

    /// Prints ANSI colored text to specific frame.
    pub fn print_ansi(&mut self, frame: usize, col: usize, row: usize, line: &str) {
        let mut col = col;
        for cell in parse_ansi_line(line, self) {
            self.frames.set(frame, col, row, cell);
            col += 1;
        }
    }
}

// Header passthrough
impl Art {
    /// Returns a title line combining the title and authors, if present.
    pub fn title_line(&self) -> String {
        self.header.title_line()
    }

    /// Returns a comma‑separated string of all authors (original and current).
    pub fn authors_line(&self) -> String {
        self.header.authors_line()
    }

    /// Removes all tags from the header.
    pub fn remove_all_tags(&mut self) {
        self.header.remove_all_tags();
    }

    /// Removes a specific tag from all tag lines.
    pub fn remove_tag(&mut self, tag: &str) {
        self.header.remove_tag(tag);
    }

    /// Adds a tag to the first tag line, or creates a new tag line if none exist.
    pub fn add_tag(&mut self, tag: &str) {
        self.header.add_tag(tag);
    }

    /// Returns a set of all tags present in the header.
    pub fn tags(&self) -> HashSet<String> {
        self.header.tags()
    }

    /// Checks if the header contains a specific tag.
    pub fn contains_tag(&self, tag: &str) -> bool {
        self.header.contains_tag(tag)
    }

    /// Removes all comments from the header, including those attached to fields,
    /// tags, and extra keys.
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

    /// Returns the color pair associated with a given character.
    pub fn get_color_map(&self, name: Char) -> ColorPair {
        self.header.get_color_map(name)
    }

    /// Sets the color pair for a character in the palette.
    pub fn set_color_map(&mut self, name: Char, col: ColorPair) {
        self.header.set_color_map(name, col);
    }

    /// Removes the color mapping for a character.
    pub fn remove_color_map(&mut self, name: Char) {
        self.header.remove_color_map(name);
        self.frames.remove_color(name);
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

    /// Checks if a name is an original author and/or current author.
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

    /// Returns whether the animation should loop (default true).
    pub fn get_loop_key(&self) -> bool {
        if let Some(flag) = self.header.loop_flag {
            flag
        } else {
            true
        }
    }

    /// Sets the loop flag.
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

    /// Gets the global delay in milliseconds (default 50).
    pub fn get_global_delay(&self) -> usize {
        if let Some(delay) = &self.header.delay {
            delay.get_global()
        } else {
            50
        }
    }

    /// Gets the delay for a specific frame in milliseconds.
    pub fn get_frame_delay(&self, frame: usize) -> usize {
        if let Some(delay) = &self.header.delay {
            delay.get_frame(frame)
        } else {
            50
        }
    }

    /// Sets the global delay.
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

    /// Sets the delay for a specific frame.
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

    /// Resets delays, optionally replacing with a new Delay object.
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

    /// Checks if any cell matching the given cell exists in any frame.
    pub fn contains(&self, cell: Cell) -> bool {
        self.frames.contains(cell)
    }

    /// Checks if any cell contains the given text character.
    pub fn contains_text(&self, ch: Char) -> bool {
        self.frames.contains_text(ch)
    }

    /// Checks if the given color name is used in the header color map or any frame.
    pub fn contains_color(&self, name: Char) -> bool {
        self.header.contains_color(name) || self.frames.contains_color(name)
    }

    /// Finds an unused character name for a new color mapping.
    pub fn free_color_name(&self) -> Char {
        // TODO: Clean up this mess

        // Try some well known chars
        let sets = vec![
            "ghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "_-+,.~?!@#$%^&*`<>()[]{}\"'\\|/:;",
            "_0123456789abcdef",
            "№¢£¥€°±÷¶§µ•…¬≈≠≤≥∞∆∂∑∏∫√",

            // Geometric Shapes
            "■□▢▣▤▥▦▧▨▩▪▫▬▭▮▯▰▱▲△▴▵▶▷▸▹►▻▼▽▾▿◀◁◂◃◄◅◆◇◈◉◊○◌◍◎●◐◑◒◓◔◕◖◗◘◙◚◛◜◝◞◟◠◡◢◣◤◥◦◧◨◩◪◫◬◭◮◰◰◱◲◳◴◵◶◷◸◸◹◺◻◼◽◾◿",

            // Block Elements
            "▀▁▂▃▄▆▇▅█▉▊▋▌▍▎▏▐░▒▓▔▕▖▗▘▙▚▛▜▝▞▟",

            // Symbols for Legacy Computing
            "🬀🬁🬂🬃🬄🬅🬆🬇🬈🬉🬊🬋🬌🬍🬎🬏🬐🬑🬒🬓🬔🬕🬖🬗🬘🬙🬚🬛🬜🬝🬞🬟🬠🬡🬢🬣🬤🬥🬦🬨🬩🬪🬫🬬🬭🬮🬯🬰🬱🬲🬳🬴🬵🬶🬷🬸🬹🬺🬻🬼🬼🬽🬾🬿🭀🭁🭂🭃🭄🭅🭆🭇🭈🭉🭊🭋🭌🭍🭎🭏🭐🭑🭒🭓🭔🭕🭖🭗🭘🭙🭚🭛🭜🭝🭞🭟🭠🭡🭢🭣🭤🭥🭦🭧🭨🭩🭪🭫🭬🭭🭮🭯🭰🭱🭲🭳🭴🭵🭶🭷🭸🭹🭺🭻🭼🭽🭾🭿🮀🮁🮂🮃🮄🮅🮇🮈🮉🮊🮋🮌🮌🮍🮎🮏🮐🮑🮒🮔🮕🮕🮖🮗🮘🮙🮚🮛🮜🮝🮞🮟🮆🮠🮡🮢🮣🮤🮥🮦🮧🮩🮨🮪🮫🮬🮭🮮🮯🮰🮱🮴🮵🮶🮷🮸🮼🮻🮾🮽🮿🯌🯎🯏🯐🯑🯒🯓🯔🯕🯖🯗🯘🯙🯚🯛🯜🯝🯞🯟🯠🯡🯢🯣🯤🯥🯦🯧🯨🯩🯪🯫🯬🯭🯮🯯🯰🯱🯲🯳🯴🯵🯶🯷🯸🯹",

            // Braille Patterns
            "⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿",

            // Enclosed Alphanumerics
            "①②③④⑤⑥⑦⑧⑨⑩⑪⑫⑬⑭⑮⑯⑰⑱⑲⑳⑴⑵⑶⑷⑸⑹⑺⑻⑼⑽⑾⑿⒀⒁⒂⒃⒄⒅⒆⒇⒈⒉⒊⒋⒌⒍⒎⒏⒐⒑⒒⒓⒔⒕⒖⒗⒘⒙⒚⒛⒜⒝⒞⒟⒠⒡⒢⒣⒤⒥⒦⒧⒨⒩⒪⒫⒬⒭⒮⒯⒰⒱⒲⒳⒴⒵ⒶⒷⒸⒹⒺⒻⒼⒽⒾⒿⓀⓁⓂⓃⓄⓅⓆⓇⓈⓉⓊⓋⓌⓍⓎⓏⓐⓑⓒⓓⓔⓕⓖⓗⓘⓙⓚⓛⓜⓝⓞⓟⓠⓡⓢⓣⓤⓥⓦⓧⓨⓩ⓪⓫⓬⓭⓮⓯⓰⓱⓲⓳⓴⓵⓶⓷⓸⓹⓺⓻⓼⓽⓾⓿",

            // Mathematical Operators
            "∀∁∂∃∄∅∆∇∈∉∊∋∌∍∎∏∐∑−∓∔∕∗∘∙√∛∜∝∞∟∠∡∢∣∤∥∦∧∨∩∪∫∬∭∮∯∰∱∲∳∴∵∶∷∸∹∺∻∼∽∾∿≀≁≂≃≄≅≆≇≈≉≊≋≌≍≎≏≐≑≒≓≔≕≖≗≘≙≚≛≜≝≞≟≠≡≢≣≤≥≦≧≨≩≪≫≬≭≮≯≰≱≲≳≴≵≶≷≸≹≺≻≼≽≾≿⊀⊁⊂⊃⊄⊅⊆⊇⊈⊉⊊⊋⊌⊍⊎⊏⊐⊑⊒⊓⊔⊕⊖⊗⊘⊙⊚⊛⊜⊝⊞⊟⊠⊡⊢⊣⊤⊥⊦⊧⊨⊩⊪⊫⊬⊭⊮⊯⊰⊱⊲⊳⊴⊵⊶⊷⊸⊹⊺⊻⊼⊽⊾⊿⋀⋁⋂⋃⋄⋅⋆⋇⋈⋉⋊⋋⋌⋍⋎⋏⋐⋑⋒⋓⋔⋕⋖⋗⋘⋙⋚⋛⋜⋝⋞⋟⋠⋡⋢⋣⋤⋥⋦⋧⋨⋩⋪⋫⋬⋭⋮⋯⋰⋱⋲⋳⋴⋵⋶⋷⋸⋹⋺⋻⋼⋽⋾⋿",

            // Arrows
            "←↑→↓↔↕↖↗↘↙↚↛↜↝↞↟↠↡↢↣↤↥↦↧↨↩↪↫↬↭↮↯↰↱↲↳↴↵↶↷↸↹↺↻↼↽↾↿⇀⇁⇂⇃⇄⇅⇆⇇⇈⇉⇊⇋⇌⇍⇎⇏⇐⇑⇒⇓⇔⇕⇖⇗⇘⇙⇚⇛⇜⇝⇞⇟⇠⇡⇢⇣⇤⇥⇦⇧⇨⇩⇪⇫⇬⇭⇮⇯⇰⇱⇲⇳⇴⇵⇶⇷⇸⇹⇺⇻⇼⇽⇾⇿",

            // Supplemental Arrows-A
            "⟰⟱⟲⟳⟴⟵⟶⟷⟸⟹⟺⟻⟼⟽⟾⟿",

            // Supplemental Arrows-B
            "⤀⤁⤂⤃⤄⤅⤆⤇⤈⤉⤊⤋⤌⤍⤎⤏⤐⤑⤒⤓⤔⤕⤖⤗⤘⤙⤚⤛⤜⤝⤞⤟⤠⤡⤢⤣⤤⤥⤦⥀⥁⥇⥈⥊⥋⥌⥍⥎⥏⥐⥑⥒⥓⥔⥕⥖⥗⥘⥙⥚⥛⥜⥝⥞⥟⥠⥡⥢⥣⥤⥥⥮⥯",

            // Supplemental Arrows-C
            "🠀🠁🠂🠃🠄🠅🠆🠇🠈🠉🠊🠋🠌🠍🠎🠏🠐🠑🠒🠓🠔🠕🠖🠗🠘🠙🠚🠛🠜🠝🠞🠟🠠🠡🠢🠣🠤🠥🠦🠧🠨🠩🠪🠫🠬🠭🠮🠯🠰🠱🠲🠳🠴🠵🠶🠷🠸🠹🠺🠻🠼🠽🠾🠿🡀🡁🡂🡃🡄🡅🡆🡇🡐🡑🡒🡓🡔🡕🡖🡗🡘🡙🡠🡡🡢🡣🡤🡥🡦🡧🡨🡩🡪🡫🡬🡭🡮🡯🡰🡱🡲🡳🡴🡵🡶🡷🡸🡹🡺🡻🡼🡽🡾🡿🢀🢁🢂🢃🢄🢅🢆🢇🢰🢱🢲🢳🢴🢵🢶🢷🢸🢹🢺🢻🣀🣁",


            // Miscellaneous Mathematical Symbols-A
            "⟀⟁⟂⟃⟄⟅⟆⟇⟈⟉⟊⟋⟌⟍⟎⟏⟐⟑⟒⟓⟔⟕⟖⟗⟘⟙⟚⟛⟜⟝⟞⟟⟠⟡⟢⟣⟤⟥⟦⟧⟨⟩⟪⟫⟮⟯",

            // Miscellaneous Mathematical Symbols-B
            "⦀⦁⦂⦃⦄⦅⦆⦇⦈⦉⦊⦋⦌⦑⦒⦙⦚⦛⦝⦞⦠⦡⦢⦣⦤⦥⦰⦱⦲⦳⦴⦵⦶⦷⦸⦹⦺⦻⦼⦽⦾⦿⧄⧅⧆⧇⧈⧉⧊⧋⧌⧍⧎⧏⧐⧑⧒⧓⧔⧕⧖⧗⧘⧙⧚⧛⧟⧢⧣⧤⧥⧦⧧⧨⧩⧪⧫⧴⧵⧶⧷⧸⧹⧺⧻",

            // Supplemental Mathematical Operators
            "⨀⨅⨆⨉⨑⨒⨓⨔⨕⨖⨝⨞⨟⨢⨣⨤⨥⨦⨧⨪⨫⨬⨭⨮⨯⨰⨱⨲⨴⨵⨶⨸⨹⨺⨻⨼⨽⨾⨿⩀⩁⩂⩃⩌⩍⩎⩏⩓⩔⩗⩘⩚⩛⩞⩟⩠⩡⩢⩣⩤⩥⩦⩧⩨⩩⩪⩫⩬⩭⩮⩯⩰⩱⩲⩳⩽⩾⩿⪀⪅⪆⪇⪈⪉⪊⪍⪎⪕⪖⪗⪘⪙⪚⪝⪞⪪⪫⪬⪭⪮⪯⪰⪱⪲⪳⪴⪵⪶⪷⪸⪹⪺⪽⪾⫏⫐⫑⫒⫙⫚⫛⫝̸⫝⫞⫟⫠⫢⫣⫤⫥⫦⫧⫨⫩⫪⫫⫬⫭⫮⫯⫰⫱⫲⫳⫴⫵⫶⫼⫽",

            // Geometric Shapes Extended
            "🞀🞁🞂🞃🞄🞅🞆🞇🞈🞉🞊🞋🞌🞍🞎🞏🞐🞑🞒🞓🞔🞕🞖🞗🞘🞙🞚🞛🞜🞝🞞🞟🞠🞡🞢🞣🞤🞥🞦🞧🞨🞨🞩🞪🞫🞬🞭🞮🞯🞯🞰🞱🞲🞳🞴🞵🞵🞶🞷🞸🞹🞺🞻🞻🞼🞽🞾🞿🟠🟡🟢🟣🟤🟥🟦🟧🟨🟩🟪🟫",

            // Latin-1 Supplement
            "¡¢£¤¥¦§¨©ª«¬®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÛÚÜÝÞßàáâãäåæèéêëìíîïðñòóôõö÷øùúûüýþÿ",

            // Latin Extended-A
            "ĀāĂăĄąĆćĈĉĊċČčĎďĐđĒēĔĕĖėĘęĚěĜĝĞğĠġĢģĤĥĦħĨĩĪīĬĭĮįİıĲĳĴĵĶķĸĹĺĻļĽľĿŀŁłŃńŅņŇňŉŊŋŌōŎŏŐőŒœŔŕŖŗŘřŚśŜŝŞşŠšŢţŤťŦŧŨũŪūŬŭŮůŰűŲųŴŵŶŷŸŹźŻżŽžſ",

            "■□●○▲△▼▽▶▷◀◁◆◇★☆❤♡♠♤♣♧♦♢",
            "←↑→↓↔↕↖↗↘↙⇐⇑⇒⇓⇔⇕↜↝",
            "αβγδζεηΘλξΞπστφωΩ",
            "бгдёилпуфцчшъыэюя",

            // Box Drawing
            "─━│┃┄┄┅┆┇┈┉┊┋┌┍┎┏┐┑┒┓└┕┖┗┘┙┚┛├┝┞┟┠┡┢┣┤┥┦┧┨┩┪┫┬┭┮┯┰┱┲┳┴┵┶┷┸┹┺┻┼┽┾╁╂╃╄╅╆╇╈╉╊╋╌╍╎╏═║╒╓╔╕╖╗╘╙╚╛╜╝╞╡╢╣╤╥╦╧╨╩╪╫╬╭╭╮╯╰╱╲╟╳╴╴╵╶╷╸╹╺╻╼╼╽╾╿",

            "ᚠᚢᚤᚣᚥᚦᚧᚨᚩᚫᚬᚭᚮᚯᚱᚳᚴᚸᚹᚻᚼᚽᚾᛃᛄᛇᛈᛉᛊᛋᛔᛗᛘᛗᛙᛜᛝᛟᛢᛣᛥᛦᛪ",
        ];
        for set in sets {
            for name in set.chars() {
                if let Ok(name) = Char::new(name) {
                    if !self.contains_color(name) {
                        return name;
                    }
                }
            }
        }
        // for name in
        //     "ghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-+,.~?!@#$%^&*`<>()[]{}\"'\\|/:;"
        //         .chars()
        // {
        //     if let Ok(name) = Char::new(name) {
        //         if !self.contains_color(name) {
        //             return name;
        //         }
        //     }
        // }
        // for name in "abcdef№¢£¥€°±÷¶§µ•…¬≈≠≤≥∞∆∂∑∏∫√■□●○▲△▼▽▶▷◀◁◆◇★☆❤♡♠♤♣♧♦♢←↑→↓↔↕↖↗↘↙⇐⇑⇒⇓⇔⇕↜↝αβγδζεηΘλξΞπστφωΩбгдёилпуфцчшъыэюяᚠᚢᚤᚣᚥᚦᚧᚨᚩᚫᚬᚭᚮᚯᚱᚳᚴᚸᚹᚻᚼᚽᚾᛃᛄᛇᛈᛉᛊᛋᛔᛗᛘᛗᛙᛜᛝᛟᛢᛣᛥᛦᛪ".chars() {
        //     if let Ok(name) = Char::new(name) {
        //         if !self.contains_color(name) {
        //             return name;
        //         }
        //     }
        // }
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

    /// Sets the entire palette.
    pub fn set_palette(&mut self, palette: Palette) {
        self.header.palette = palette
    }

    /// Resets the palette to default.
    pub fn remove_palette(&mut self) {
        self.header.palette = Palette::default();
    }

    /// Searches for a color pair in the color map and returns its character name if found.
    pub fn search_color_map(&self, col: ColorPair) -> Option<Char> {
        self.header.search_color_map(col)
    }

    /// Searches for a color pair, creating a new mapping if not found.
    pub fn search_or_create_color_map(&mut self, col: ColorPair) -> Char {
        if let Some(name) = self.search_color_map(col) {
            name
        } else {
            let name = self.free_color_name();
            self.set_color_map(name, col);
            name
        }
    }

    /// Removes a frame at the given index.
    pub fn remove_frame(&mut self, frame: usize) {
        self.frames.remove_frame(frame);
    }

    /// Ensures a frame exists at the given index, creating new frames if necessary.
    pub fn make_sure_frame_exist(&mut self, frame: usize) {
        self.frames.make_sure_frame_exist(frame);
    }

    /// Duplicates a frame, inserting the copy after the original.
    pub fn dup_frame(&mut self, frame: usize) {
        self.frames.dup_frame(frame);
    }
}

// Conversions
impl Art {
    /// Returns the total duration of the animation in seconds.
    pub fn duration(&self) -> f64 {
        let mut dur: usize = 0;
        for f in 0..self.frames() {
            dur += self.get_frame_delay(f);
        }
        dur as f64 / 1000.0
    }

    /// Converts the art to json document with extra metadata
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n");

        // Metadata
        let (text_pinned, color_pinned) = self.pinned();
        json += &format!(
            "  \"meta\": {{\n    \"frames\": {},\n    \"width\": {},\n    \"height\": {},\n    \"duration\": {},\n    \"text-pinned\": {},\n    \"color-pinned\": {}\n  }},\n",
            self.frames(),
            self.width(),
            self.height(),
            self.duration(),
            text_pinned,
            color_pinned
        );

        // Header
        json += "  \"header\": {\n";
        if let Some(title) = &self.header.title {
            json += &format!("    \"title\": {},\n", json_quote(title));
        } else {
            json += "    \"title\": null,\n"
        }
        if self.header.authors.len() > 0 {
            json += "    \"authors\": [\n";
            for (i, author) in self.header.authors.keys().enumerate() {
                if i < self.header.authors.len() - 1 {
                    json += &format!("      {},\n", json_quote(author));
                } else {
                    json += &format!("      {}\n", json_quote(author));
                }
            }
            json += "    ],\n";
        } else {
            json += "    \"authors\": [],\n";
        }
        if self.header.orig_authors.len() > 0 {
            json += "    \"orig-authors\": [\n";
            for (i, author) in self.header.orig_authors.keys().enumerate() {
                if i < self.header.orig_authors.len() - 1 {
                    json += &format!("      {},\n", json_quote(author));
                } else {
                    json += &format!("      {}\n", json_quote(author));
                }
            }
            json += "    ],\n";
        } else {
            json += "    \"orig-authors\": [],\n";
        }
        if let Some(src) = &self.header.src {
            json += &format!("    \"src\": {},\n", json_quote(src));
        } else {
            json += "    \"src\": null,\n";
        }
        if let Some(editor) = &self.header.editor {
            json += &format!("    \"editor\": {},\n", json_quote(editor));
        } else {
            json += "    \"editor\": null,\n";
        }
        json += &format!(
            "    \"license\": {},\n",
            json_quote(&(self.header.license.clone().unwrap_or("proprietary".into())))
        );
        json += &format!("    \"loop\": {},\n", self.get_loop_key());
        json += &format!("    \"preview\": {},\n", self.header.preview.unwrap_or(0));
        json += &format!("    \"colors\": {},\n", self.color());
        json += "    \"palette\": {";
        for c in "_0123456789abcdef".chars() {
            let pair = self.get_color_map(Char::new_must(c));
            json += &format!(
                "{}\n      {}: {{ \"fg\": {}, \"bg\": {} }}",
                if c == '_' { "" } else { "," },
                json_quote(&String::from(c)),
                json_quote(&pair.fg.to_string()),
                json_quote(&pair.bg.to_string()),
            );
        }
        for c in self.header.palette.palette.keys() {
            if "_0123456789abcdef".contains(c.char) {
                continue;
            }
            let pair = self.get_color_map(*c);
            json += &format!(
                ",\n      {}: {{ \"fg\": {}, \"bg\": {} }}",
                json_quote(&c.to_string()),
                json_quote(&pair.fg.to_string()),
                json_quote(&pair.bg.to_string()),
            );
        }
        json += "\n    },\n";
        let tags = self.tags();
        let tags_len = tags.len();
        if tags.len() > 0 {
            json += "    \"tags\": [\n";
            for (i, tag) in tags.into_iter().enumerate() {
                if i < tags_len - 1 {
                    json += &format!("      {},\n", json_quote(&tag));
                } else {
                    json += &format!("      {}\n", json_quote(&tag));
                }
            }
            json += "    ],\n";
        } else {
            json += "    \"tags\": [],\n";
        }
        if self.header.extra_keys.len() > 0 {
            json += "    \"extra-keys\": [\n";
            for (i, key) in self.header.extra_keys.iter().enumerate() {
                if i < self.header.extra_keys.len() - 1 {
                    json += &format!("      {},\n", json_quote(&key.line));
                } else {
                    json += &format!("      {}\n", json_quote(&key.line));
                }
            }
            json += "    ]\n";
        } else {
            json += "    \"extra-keys\": []\n";
        }
        json += "  },\n";

        // Attached
        json += &format!(
            "  \"attached\": {},\n",
            if let Some(a) = &self.attached {
                json_quote(a)
            } else {
                String::from("null")
            }
        );

        // Extra
        if self.extra.len() > 0 {
            json += "  \"extra-blocks\": [\n";
            for (i, block) in self.extra.iter().enumerate() {
                if i < self.extra.len() - 1 {
                    json += &format!(
                        "    {{ \"title\": {}, \"content\": {} }},\n",
                        json_quote(&block.title),
                        json_quote(&block.content)
                    );
                } else {
                    json += &format!(
                        "    {{ \"title\": {}, \"content\": {} }}\n",
                        json_quote(&block.title),
                        json_quote(&block.content)
                    );
                }
            }
            json += "  ],\n";
        } else {
            json += "  \"extra-blocks\": [],\n";
        }

        // Frames
        json += "  \"frames\": [\n";
        for (f, frame) in self.frames.frames.iter().enumerate() {
            json += &format!("    {{\n      \"delay\": {},\n", self.get_frame_delay(f));
            json += "      \"text\": [\n";
            for (r, row) in frame.rows.iter().enumerate() {
                let mut rowstr = String::new();
                for cell in row {
                    rowstr.push(cell.text.char);
                }
                if r + 1 < frame.rows.len() {
                    json += &format!("        {},\n", json_quote(&rowstr))
                } else {
                    json += &format!("        {}\n", json_quote(&rowstr))
                }
            }
            json += "      ],\n      \"colors\": [\n";
            for (r, row) in frame.rows.iter().enumerate() {
                let mut rowstr = String::new();
                for cell in row {
                    rowstr.push(cell.color.unwrap_or(UNDERSCORE).char);
                }
                if r + 1 < frame.rows.len() {
                    json += &format!("        {},\n", json_quote(&rowstr))
                } else {
                    json += &format!("        {}\n", json_quote(&rowstr))
                }
            }
            json += "      ]\n";
            if f + 1 < self.frames() {
                json += "    },\n";
            } else {
                json += "    }\n";
            }
        }
        json += "  ]\n}\n";
        json
    }

    /// Converts the art to durformat
    pub fn to_dur(&self) -> String {
        let mut dur = String::from(
            "{\n  \"DurMovie\": {\n    \"formatVersion\": 7,\n    \"colorFormat\": \"256\",\n    \"preferredFont\": \"fixed\",\n    \"encoding\": \"utf-8\",\n    \"extra\": null,\n    \"framerate\": 60.0,\n",
        );
        dur += &format!(
            "    \"name\": {},\n",
            json_quote(&self.get_title_key().unwrap_or(String::new())),
        );
        dur += &format!("    \"artist\": {},\n", json_quote(&self.authors_line()));
        dur += &format!("    \"sizeX\": {},\n", self.width());
        dur += &format!("    \"sizeY\": {},\n", self.height());
        dur += "    \"frames\":[\n";
        for (i, frame) in self.frames.frames.iter().enumerate() {
            dur += "      {\n";
            dur += &format!("        \"frameNumber\": {},\n", i + 1);
            dur += &format!(
                "        \"delay\": {},\n",
                self.get_frame_delay(i) as f64 / 1000.0
            );
            dur += "        \"contents\": [\n";
            for r in 0..self.height() {
                let mut row = String::new();
                for c in 0..self.width() {
                    row.push(frame.get(c, r, Cell::default()).text.char);
                }
                row = json_quote(&row);
                if r + 1 < self.height() {
                    dur += &format!("          {},\n", row);
                } else {
                    dur += &format!("          {}\n", row);
                }
            }
            dur += "        ],\n";
            dur += "        \"colorMap\": [\n";
            for c in 0..self.width() {
                let mut line = String::new();
                for r in 0..self.height() {
                    let fg = if let Some(name) = frame.get(c, r, Cell::default()).color {
                        self.get_color_map(name).fg.to_durdraw_color()
                    } else {
                        0
                    };
                    if r + 1 < self.height() {
                        line += &format!("[{}, 0],", fg);
                    } else {
                        line += &format!("[{}, 0]", fg);
                    }
                }
                if c + 1 < self.width() {
                    dur += &format!("          [{}],\n", line);
                } else {
                    dur += &format!("          [{}]\n", line);
                }
            }
            dur += "        ]\n";
            if i + 1 < self.frames() {
                dur += "      },\n";
            } else {
                dur += "      }\n";
            }
        }
        dur += "    ]\n  }\n}\n";
        dur
    }

    /// Converts the art to ASCIIcast v2 format string.
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

    /// Converts the art to an SVG frames string using the given CSS color map and font.
    pub fn to_svg_frames(&self, map: &CSSColorMap, font: &Font) -> String {
        let delay = self.header.delay.clone().unwrap_or(Delay::default());
        self.frames
            .to_svg_frames(self.color(), &self.header.palette, map, font, &delay)
    }

    /// Returns a vector of ANSI-encoded strings for each frame.
    pub fn to_ansi_frames(&self) -> Vec<String> {
        self.frames
            .to_ansi_frames(&self.header.palette, self.color())
    }

    /// Returns a single ANSI string concatenating all frames with default color reset at the end.
    pub fn to_ansi_string(&self) -> String {
        format!(
            "{}{}\n",
            self.to_ansi_frames().join("\n"),
            ColorPair::default().to_ansi()
        )
    }

    /// Writes the ANSI representation to a file.
    pub fn to_ansi_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        writeln!(
            file,
            "{}{}",
            self.to_ansi_frames().join("\n"),
            ColorPair::default().to_ansi()
        )
    }

    /// Writes the native 3a format to a file.
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        write!(file, "{}", self)
    }

    /// Consumes the art and returns its components: header, frames, attached, extra.
    pub fn to_components(self) -> (Header, Frames, Option<String>, Vec<ExtraBlock>) {
        (self.header, self.frames, self.attached, self.extra)
    }

    pub fn to_ttyrec(&self) -> Vec<u8> {
        let mut v = Vec::new();
        let mut delay_acum: usize = 0;
        for (f, frame) in self.frames.frames.iter().enumerate() {
            let mut text = frame.ansi(&self.header.palette, self.color());
            if f == 0 {
                text += &format!("\x1b]0;{}\x07", self.title_line());
            }
            text += &format!("\x1b[{}A\n", self.height());
            let tf = TtyrecFrame {
                timestamp_ms: delay_acum,
                text: text,
            };
            delay_acum += self.get_frame_delay(f);
            tf.append_to_vec(&mut v);
        }
        if self.frames() > 0 {
            let f = self.frames() - 1;
            let frame = &self.frames.frames[f];
            let mut text = frame.ansi(&self.header.palette, self.color());
            text += "\n";
            let tf = TtyrecFrame {
                timestamp_ms: delay_acum,
                text: text,
            };
            tf.append_to_vec(&mut v);
        }
        v
    }

    /// Creates an Art from its components.
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

    /// Converts ttyrec anomation to Art
    pub fn from_ttyrec<R: Read>(reader: R) -> Result<Self> {
        let mut a = Self::new(1, 10, 10, Cell::default());
        let mut max_width: usize = 0;
        let mut max_height: usize = 0;
        let mut frames: Vec<Vec<Vec<Cell>>> = vec![];
        for raw in TtyrecReader::new(reader) {
            let raw = raw?;
            let mut frame: Vec<Vec<Cell>> = vec![];
            for line in raw.text.lines() {
                let row = parse_ansi_line(line, &mut a);
                max_width = max_width.max(row.len());
                frame.push(row);
            }
            max_height = max_height.max(frame.len());
            frames.push(frame);
        }
        let mut art = Self::new(frames.len(), max_width, max_height, Cell::default());
        for (f, frame) in frames.iter().enumerate() {
            for (r, row) in frame.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    art.set(f, c, r, *cell);
                }
            }
        }
        Ok(art)
    }

    /// Constructs art from plain text with ANSI color escape codes.
    pub fn from_ansi_text(text: &str) -> Self {
        let mut a = Self::new(1, 10, 10, Cell::default());
        let mut rows: Vec<Vec<Cell>> = Vec::new();
        let mut max_width: usize = 0;
        for line in text.lines() {
            let row = parse_ansi_line(line, &mut a);
            max_width = max_width.max(row.len());
            rows.push(row);
        }
        let mut art = Self::new(1, max_width, rows.len(), Cell::default());
        art.header.palette = a.header.palette;
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                art.set(0, c, r, *cell);
            }
        }
        art
    }

    /// Reads an Art from a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_reader(File::open(path)?)
    }

    /// Reads an Art from any reader.
    pub fn from_reader<R: Read>(r: R) -> Result<Self> {
        let mut lines = BufReader::new(r).lines();
        Self::from_lines(&mut lines)
    }

    /// Reads an Art from an iterator of lines.
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
    /// Writes the art in its native 3a format.
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

/// An extra block in the 3a file format with a title and content.
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

pub(crate) fn parse_ansi_line(line: &str, art: &mut Art) -> Vec<Cell> {
    let mut out = Vec::new();
    let mut iter = line.char_indices().peekable();

    let mut fg = Color::None;
    let mut bg = Color::None;

    while let Some((_idx, ch)) = iter.next() {
        if ch == '\x1b' {
            // If there's a next char, inspect it
            if let Some(&(_, next_ch)) = iter.peek() {
                match next_ch {
                    '[' => {
                        // CSI — consume '[' and parse until 'm'
                        iter.next(); // consume '['

                        // collect until 'm' (SGR) or end
                        let mut params = String::new();
                        let mut saw_m = false;
                        while let Some(&(_, c)) = iter.peek() {
                            iter.next();
                            if c == 'm' {
                                saw_m = true;
                                break;
                            } else {
                                params.push(c);
                            }
                        }

                        if saw_m {
                            let nums: Vec<i32> = if params.is_empty() {
                                vec![]
                            } else {
                                params
                                    .split(';')
                                    .map(|s| s.parse::<i32>().unwrap_or(-999))
                                    .collect()
                            };
                            apply_sgr(&nums, &mut fg, &mut bg);
                        }
                        // whether saw_m or not, skip the whole CSI sequence
                        continue;
                    }

                    ']' => {
                        // OSC — consume ']' and skip until BEL (\x07) or ST (ESC \)
                        iter.next(); // consume ']'

                        loop {
                            match iter.next() {
                                None => break, // unterminated OSC — give up at end-of-string
                                Some((_i, c2)) => {
                                    if c2 == '\x07' {
                                        // BEL terminates OSC
                                        break;
                                    }
                                    if c2 == '\x1b' {
                                        // could be ESC \ (ST). Peek next char
                                        if let Some(&(_, maybe_backslash)) = iter.peek() {
                                            if maybe_backslash == '\\' {
                                                // consume backslash and finish OSC
                                                iter.next();
                                                break;
                                            } else {
                                                // It's an ESC followed by something else — continue skipping
                                                continue;
                                            }
                                        } else {
                                            // ESC at end — unterminated, stop
                                            break;
                                        }
                                    }
                                    // otherwise keep skipping characters
                                }
                            }
                        }

                        continue;
                    }

                    // Other ESC sequences we don't process (DCS, SOS, PM, etc.)
                    // For now: just skip the ESC itself and continue (don't consume the following char here).
                    _ => {
                        // Don't consume the next_ch here — treat ESC as skipped non-printable.
                        // If you want to recognize more control sequences, add cases here.
                        continue;
                    }
                }
            } else {
                // ESC at end-of-input — ignore
                continue;
            }
        }

        if let Ok(ch) = Char::new(ch) {
            let color = if fg != Color::None || bg != Color::None {
                let color = art.search_or_create_color_map(ColorPair { fg, bg });
                Some(color)
            } else {
                None
            };
            out.push(Cell {
                text: ch,
                color: color,
            });
        }
    }

    out
}

#[test]
fn fg_and_bg() {
    let mut art = Art::new(1, 10, 10, Cell::default());
    let s = "\x1b[0m\x1b]0;GAY\x1b\\\x1b[31;44m\x1b[38;2;7;214;105mA\x1b[0mB";
    let v = parse_ansi_line(s, &mut art);

    assert_eq!(v[0].text.char, 'A');
    let pair = art.get_color_map(v[0].color.unwrap());
    assert_eq!(pair.fg, Color::RGB(7, 214, 105));
    assert_eq!(pair.bg, Color::Color4(Color4::Blue, false));

    assert_eq!(v[1].text.char, 'B');
    assert_eq!(v[1].color, None);
}

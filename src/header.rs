use crate::comments::write_comments;
use core::fmt;
use std::{
    collections::HashSet,
    fmt::Display,
    io::{self, BufRead, BufReader, Cursor, Read},
    str::FromStr,
};

use ordermap::{OrderMap, OrderSet};

use crate::error::{Error, Result};
use crate::{
    chars::{normalize_text, Char},
    comments::Comments,
};
use crate::{delay::Delay, ColorPair, Palette};

/// Represents the header of a 3a file.
#[derive(Default, Debug, Clone)]
pub struct Header {
    /// Optional title of the artwork.
    pub title: Option<String>,
    /// Comments associated with the title.
    pub title_comments: Comments,

    /// Map of author names to their associated comments.
    pub authors: OrderMap<String, Comments>,

    /// Map of original author names to their comments.
    pub orig_authors: OrderMap<String, Comments>,

    /// Optional source URL or description.
    pub src: Option<String>,
    /// Comments associated with the source.
    pub src_comments: Comments,

    /// Optional editor name.
    pub editor: Option<String>,
    /// Comments associated with the editor.
    pub editor_comments: Comments,

    /// Optional license information.
    pub license: Option<String>,
    /// Comments associated with the license.
    pub license_comments: Comments,

    /// Optional frame delay for animations.
    pub delay: Option<Delay>,
    /// Comments associated with the delay.
    pub delay_comments: Comments,

    /// Optional loop flag (true = loop, false = no loop).
    pub loop_flag: Option<bool>,
    /// Comments associated with the loop flag.
    pub loop_comments: Comments,

    /// Optional preview frame index.
    pub preview: Option<usize>,
    /// Comments associated with the preview.
    pub preview_comments: Comments,

    /// Optional flag indicating presence of colors.
    pub colors: Option<bool>,
    /// Comments associated with the colors flag.
    pub colors_comments: Comments,

    /// Color palette mapping characters to color pairs.
    pub palette: Palette,

    /// List of tag lines, each containing a set of tags and comments.
    pub tags: Vec<Tagline>,

    /// Legacy header information for compatibility with older format version.
    pub legacy: Option<LegacyHeaderInfo>,

    /// Extra unrecognized header keys preserved for round‑tripping.
    pub extra_keys: Vec<ExtraHeaderKey>,

    /// Comments that appear after all header keys.
    pub trailing_comments: Comments,
}

impl Header {
    /// Removes all tags from the header.
    pub fn remove_all_tags(&mut self) {
        self.tags = Vec::new();
    }
    /// Removes a specific tag from all tag lines.
    pub fn remove_tag(&mut self, tag: &str) {
        let mut taglines: Vec<Tagline> = Vec::new();
        for tagline in self.tags.iter_mut() {
            tagline.tags.remove(tag);
            if tagline.tags.len() > 0 {
                taglines.push(tagline.clone());
            }
        }
        self.tags = taglines;
    }
    /// Adds a tag to the first tag line, or creates a new tag line if none exist.
    pub fn add_tag(&mut self, tag: &str) {
        let tag = normalize_text(tag);
        if tag.len() < 1 {
            return;
        }
        if self.contains_tag(&tag) {
            return;
        }
        if self.tags.len() > 0 {
            self.tags[0].tags.insert(tag);
        } else {
            let mut tags = OrderSet::new();
            tags.insert(tag);
            self.tags.push(Tagline {
                tags,
                comments: Vec::new(),
            });
        }
    }
    /// Returns a set of all tags present in the header.
    pub fn tags(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        for tagline in &self.tags {
            for tag in &tagline.tags {
                set.insert(tag.clone());
            }
        }
        set
    }
    /// Checks if the header contains a specific tag.
    pub fn contains_tag(&self, tag: &str) -> bool {
        for tagline in &self.tags {
            if tagline.tags.contains(tag) {
                return true;
            }
        }
        false
    }
    /// Removes all comments from the header, including those attached to fields,
    /// tags, and extra keys.
    pub fn strip_comments(&mut self) {
        self.title_comments = Vec::new();
        self.src_comments = Vec::new();
        self.editor_comments = Vec::new();
        self.license_comments = Vec::new();
        self.delay_comments = Vec::new();
        self.loop_comments = Vec::new();
        self.preview_comments = Vec::new();
        self.colors_comments = Vec::new();
        self.trailing_comments = Vec::new();

        let keys: Vec<String> = self.authors.keys().map(|k| k.clone()).collect();
        for key in keys {
            self.authors.insert(key, Vec::new());
        }
        let keys: Vec<String> = self.orig_authors.keys().map(|k| k.clone()).collect();
        for key in keys {
            self.orig_authors.insert(key, Vec::new());
        }

        for tag in self.tags.iter_mut() {
            tag.comments = Vec::new();
        }

        for key in self.extra_keys.iter_mut() {
            key.comments = Vec::new();
        }

        self.palette.strip_comments();
    }
    /// Returns whether colors are enabled, considering the colors flag and legacy mode.
    pub fn get_colors(&self) -> bool {
        if let Some(colors) = self.colors {
            colors
        } else if let Some(legacy) = self.legacy {
            legacy.colors != LegacyColorMode::None
        } else {
            self.palette.len() > 0
        }
    }
    /// Returns the color pair associated with a given character.
    pub fn get_color_map(&self, name: Char) -> ColorPair {
        self.palette.get_color(name)
    }
    /// Sets the color pair for a character in the palette.
    pub fn set_color_map(&mut self, name: Char, col: ColorPair) {
        self.palette.set_color(name, col)
    }
    /// Removes the color mapping for a character.
    pub fn remove_color_map(&mut self, name: Char) {
        self.palette.remove_color(name);
    }
    /// Searches for a character that has the given color pair.
    pub fn search_color_map(&self, col: ColorPair) -> Option<Char> {
        self.palette.search_color(col)
    }
    /// Checks if the palette contains a mapping for the given character.
    pub fn contains_color(&self, name: Char) -> bool {
        self.palette.contains_color(name)
    }
    /// Returns a comma‑separated string of all authors (original and current).
    pub fn authors_line(&self) -> String {
        self.orig_authors
            .keys()
            .chain(self.authors.keys())
            .map(|s| s.clone())
            .collect::<Vec<String>>()
            .join(", ")
    }
    /// Returns a title line combining the title and authors, if present.
    pub fn title_line(&self) -> String {
        let authors = self.authors_line();
        if let Some(s) = &self.title {
            if authors == "" {
                s.clone()
            } else {
                format!("{} by {}", s, authors)
            }
        } else {
            if authors == "" {
                String::from("")
            } else {
                format!("art by {}", authors)
            }
        }
    }
}

impl Header {
    pub(crate) fn set_legacy_mode_str(&mut self, mode: &str) {
        let mode = match mode.trim().to_lowercase().as_str() {
            "none" => LegacyColorMode::None,
            "fg" => LegacyColorMode::FgOnly,
            "bg" => LegacyColorMode::BgOnly,
            "full" => LegacyColorMode::FgAndBg,
            _ => LegacyColorMode::default(),
        };
        let mut l = match &self.legacy {
            Some(l) => l.clone(),
            None => LegacyHeaderInfo::default(),
        };
        l.colors = mode;
        self.legacy = Some(l);
    }
    pub(crate) fn set_legacy_width(&mut self, width: usize) {
        let mut l = match &self.legacy {
            Some(l) => l.clone(),
            None => LegacyHeaderInfo::default(),
        };
        l.width = width;
        self.legacy = Some(l);
    }
    pub(crate) fn set_legacy_height(&mut self, height: usize) {
        let mut l = match &self.legacy {
            Some(l) => l.clone(),
            None => LegacyHeaderInfo::default(),
        };
        l.height = height;
        self.legacy = Some(l);
    }
    /// Formats the header with explicit control over whether colors exist,
    /// used for writing.
    pub fn fmt_with_colors(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        colors_exist: bool,
    ) -> std::fmt::Result {
        writeln!(f, "@3a")?;
        if let Some(title) = &self.title {
            write_comments(&self.title_comments, f)?;
            writeln!(f, "title {}", title)?;
        }
        for (author, comments) in &self.orig_authors {
            write_comments(&comments, f)?;
            writeln!(f, "orig-author {}", author)?;
        }
        for (author, comments) in &self.authors {
            write_comments(&comments, f)?;
            writeln!(f, "author {}", author)?;
        }
        if let Some(src) = &self.src {
            write_comments(&self.src_comments, f)?;
            writeln!(f, "src {}", src)?;
        }
        if let Some(editor) = &self.editor {
            write_comments(&self.editor_comments, f)?;
            writeln!(f, "editor {}", editor)?;
        }
        if let Some(license) = &self.license {
            write_comments(&self.license_comments, f)?;
            writeln!(f, "license {}", license)?;
        }
        if let Some(delay) = &self.delay {
            write_comments(&self.delay_comments, f)?;
            writeln!(f, "delay {}", delay)?;
        }
        if let Some(flag) = &self.loop_flag {
            write_comments(&self.loop_comments, f)?;
            writeln!(f, "loop {}", if *flag { "yes" } else { "no" })?;
        }
        if let Some(preview) = &self.preview {
            write_comments(&self.preview_comments, f)?;
            writeln!(f, "preview {}", preview)?;
        }
        if let Some(colors) = self.colors {
            if colors {
                if self.palette.len() > 0 {
                    self.palette.fmt(f)?;
                } else {
                    writeln!(f, "colors yes")?;
                }
            }
        } else if colors_exist {
            if self.palette.len() > 0 {
                self.palette.fmt(f)?;
            } else {
                writeln!(f, "colors yes")?;
            }
        } else {
            self.palette.fmt(f)?;
        }
        for tagline in &self.tags {
            tagline.fmt(f)?;
        }
        write_comments(&self.trailing_comments, f)?;
        writeln!(f, "")?;
        Ok(())
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "@3a")?;
        if let Some(title) = &self.title {
            for c in &self.title_comments {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "title {}", title)?;
        }
        for (author, comments) in &self.orig_authors {
            for c in comments {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "orig-author {}", author)?;
        }
        for (author, comments) in &self.authors {
            for c in comments {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "author {}", author)?;
        }
        if let Some(src) = &self.src {
            for c in &self.src_comments {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "src {}", src)?;
        }
        if let Some(editor) = &self.editor {
            for c in &self.editor_comments {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "editor {}", editor)?;
        }
        if let Some(license) = &self.license {
            for c in &self.license_comments {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "license {}", license)?;
        }
        if let Some(delay) = &self.delay {
            for c in &self.delay_comments {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "delay {}", delay)?;
        }
        if let Some(flag) = &self.loop_flag {
            for c in &self.loop_comments {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "loop {}", if *flag { "yes" } else { "no" })?;
        }
        if let Some(preview) = &self.preview {
            for c in &self.preview_comments {
                writeln!(f, ";; {}", c)?;
            }
            writeln!(f, "preview {}", preview)?;
        }
        if let Some(colors) = self.colors {
            if colors {
                if self.palette.len() > 0 {
                    self.palette.fmt(f)?;
                } else {
                    writeln!(f, "colors yes")?;
                }
            }
        } else {
            self.palette.fmt(f)?;
        }
        for tagline in &self.tags {
            tagline.fmt(f)?;
        }
        for c in &self.trailing_comments {
            writeln!(f, ";; {}", c)?;
        }
        writeln!(f, "")?;
        Ok(())
    }
}

impl Header {
    /// Reads a header from a buffered reader, automatically detecting modern
    /// or legacy format.
    pub fn read<R: Read>(lines: &mut io::Lines<BufReader<R>>) -> Result<Self> {
        let fl = lines.next();
        if let Some(Ok(s)) = fl {
            if s == "@3a" {
                Self::read_modern(lines)
            } else {
                Self::read_legacy(s.as_str(), lines)
            }
        } else {
            Self::read_legacy("@", lines)
        }
    }
    pub(crate) fn read_legacy<R: Read>(
        first: &str,
        lines: &mut io::Lines<BufReader<R>>,
    ) -> Result<Self> {
        let mut header = Self::default();
        let mut comments_buffer = Vec::<String>::new();
        let fr = BufReader::new(Cursor::new(first.as_bytes())).lines();
        for line in fr.chain(lines) {
            let line = line?;
            if line.is_empty() {
                break;
            }
            // if let Some(comment) = line.strip_prefix("\t") {
            //     comments_buffer.push(normalize_text(comment).trim().into());
            //     continue;
            // }
            let line = match line.split_once("\t") {
                Some((a, b)) => {
                    if a.is_empty() {
                        comments_buffer.push(normalize_text(b).trim().into());
                        continue;
                    }
                    a
                }
                None => &line,
            };
            let line = normalize_text(line);
            if line.is_empty() {
                break;
            }
            if let Some(comment) = line.strip_prefix("@") {
                comments_buffer.push(comment.trim().into());
                continue;
            }
            if line.starts_with("#") {
                let mut tagline = line.parse::<Tagline>()?;
                let tl = header.tags.len();
                if tl > 0 && comments_buffer.len() == 0 {
                    for tag in tagline.tags {
                        header.tags[tl - 1].tags.insert(tag);
                    }
                } else {
                    tagline.comments = comments_buffer.clone();
                    comments_buffer.clear();
                    header.tags.push(tagline);
                }
                continue;
            }
            let err = Error::HeaderKeyWithoutValue(line.clone());
            if line.starts_with("utf8") {
                continue;
            }
            let (key, values) = line.split_once(" ").ok_or(err)?;
            let key = key.trim();
            let values = values.trim();
            match key {
                "title" => {
                    if let Some(_) = header.title {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.title = Some(values.into());
                    header.title_comments = comments_buffer.clone();
                }
                "author" => match header.authors.get(values) {
                    Some(comments) => {
                        header.authors.insert(
                            values.into(),
                            comments
                                .into_iter()
                                .map(|s| s.clone())
                                .chain(comments_buffer.clone())
                                .collect::<Vec<String>>(),
                        );
                    }
                    None => {
                        header
                            .authors
                            .insert(values.into(), comments_buffer.clone());
                    }
                },
                "loop" => {
                    if let Some(_) = header.loop_flag {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.loop_flag = Some(header_value_to_bool(key, values)?);
                    header.loop_comments = comments_buffer.clone();
                }
                "preview" => {
                    if let Some(_) = header.preview {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    match values.parse::<usize>() {
                        Ok(preview) => {
                            header.preview = Some(preview);
                            header.preview_comments = comments_buffer.clone();
                        }
                        Err(err) => {
                            return Err(Error::PreviewParsing(values.into(), err));
                        }
                    }
                }
                "delay" => {
                    if let Some(_) = header.delay {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.delay = Some(values.parse()?);
                    header.delay_comments = comments_buffer.clone();
                }
                "colors" => {
                    header.set_legacy_mode_str(values);
                }
                "width" => match values.parse::<usize>() {
                    Ok(preview) => {
                        header.set_legacy_width(preview);
                    }
                    Err(err) => {
                        return Err(Error::PreviewParsing(values.into(), err));
                    }
                },
                "height" => match values.parse::<usize>() {
                    Ok(preview) => {
                        header.set_legacy_height(preview);
                    }
                    Err(err) => {
                        return Err(Error::PreviewParsing(values.into(), err));
                    }
                },
                _ => {
                    header.extra_keys.push(ExtraHeaderKey {
                        line: String::from(key) + " " + values,
                        comments: comments_buffer.clone(),
                    });
                }
            }
            comments_buffer.clear();
        }
        header.trailing_comments = comments_buffer;
        Ok(header)
    }
    pub(crate) fn read_modern<R: Read>(lines: &mut io::Lines<BufReader<R>>) -> Result<Self> {
        let mut header = Self::default();
        let mut comments_buffer = Vec::<String>::new();
        for line in lines {
            let line = normalize_text(line?.as_str());
            if line.is_empty() {
                break;
            }
            if line == "@3a" {
                continue;
            }
            if let Some(comment) = line.strip_prefix(";;") {
                comments_buffer.push(comment.trim().into());
                continue;
            }
            if line.starts_with("#") {
                let mut tagline = line.parse::<Tagline>()?;
                let tl = header.tags.len();
                if tl > 0 && comments_buffer.len() == 0 {
                    for tag in tagline.tags {
                        header.tags[tl - 1].tags.insert(tag);
                    }
                } else {
                    tagline.comments = comments_buffer.clone();
                    comments_buffer.clear();
                    header.tags.push(tagline);
                }
                continue;
            }
            let err = Error::HeaderKeyWithoutValue(line.clone());
            let (key, values) = line.split_once(" ").ok_or(err)?;
            let key = key.trim();
            let values = values.trim();
            match key {
                "title" => {
                    if let Some(_) = header.title {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.title = Some(values.into());
                    header.title_comments = comments_buffer.clone();
                }
                "orig-author" => match header.orig_authors.get(values) {
                    Some(comments) => {
                        header.orig_authors.insert(
                            values.into(),
                            comments
                                .into_iter()
                                .map(|s| s.clone())
                                .chain(comments_buffer.clone())
                                .collect::<Vec<String>>(),
                        );
                    }
                    None => {
                        header
                            .orig_authors
                            .insert(values.into(), comments_buffer.clone());
                    }
                },
                "author" => match header.authors.get(values) {
                    Some(comments) => {
                        header.authors.insert(
                            values.into(),
                            comments
                                .into_iter()
                                .map(|s| s.clone())
                                .chain(comments_buffer.clone())
                                .collect::<Vec<String>>(),
                        );
                    }
                    None => {
                        header
                            .authors
                            .insert(values.into(), comments_buffer.clone());
                    }
                },
                "src" => {
                    if let Some(_) = header.src {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.src = Some(values.into());
                    header.src_comments = comments_buffer.clone();
                }
                "editor" => {
                    if let Some(_) = header.editor {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.editor = Some(values.into());
                    header.editor_comments = comments_buffer.clone();
                }
                "license" => {
                    if let Some(_) = header.license {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.license = Some(values.into());
                    header.license_comments = comments_buffer.clone();
                }
                "delay" => {
                    if let Some(_) = header.delay {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.delay = Some(values.parse()?);
                    header.delay_comments = comments_buffer.clone();
                }
                "loop" => {
                    if let Some(_) = header.loop_flag {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.loop_flag = Some(header_value_to_bool(key, values)?);
                    header.loop_comments = comments_buffer.clone();
                }
                "preview" => {
                    if let Some(_) = header.preview {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    match values.parse::<usize>() {
                        Ok(preview) => {
                            header.preview = Some(preview);
                            header.preview_comments = comments_buffer.clone();
                        }
                        Err(err) => {
                            return Err(Error::PreviewParsing(values.into(), err));
                        }
                    }
                }
                "colors" => {
                    if let Some(_) = header.colors {
                        return Err(Error::HeaderKeyDup(key.into()));
                    }
                    header.colors = Some(header_value_to_bool(key, values)?);
                    header.colors_comments = comments_buffer.clone();
                }
                "col" => {
                    let mut values = values.split(" ");
                    let n = values.next();
                    let name = color_name_str_to_char(n)?;
                    let strpair = values.collect::<Vec<&str>>().join(" ");
                    let pair = strpair.parse::<ColorPair>()?;

                    header
                        .palette
                        .add_parsing_color(name, pair, comments_buffer.clone())?;
                }
                _ => {
                    header.extra_keys.push(ExtraHeaderKey {
                        line: String::from(key) + " " + values,
                        comments: comments_buffer.clone(),
                    });
                }
            };
            comments_buffer.clear();
        }
        header.trailing_comments = comments_buffer;
        Ok(header)
    }
}

fn header_value_to_bool(k: &str, v: &str) -> Result<bool> {
    match v.trim().to_lowercase().as_str() {
        "yes" => Ok(true),
        "true" => Ok(true),
        "no" => Ok(false),
        "false" => Ok(false),
        _ => Err(Error::HeaderFlagKey(k.into())),
    }
}

fn color_name_str_to_char(name: Option<&str>) -> Result<Char> {
    let name = name.unwrap_or_default();
    Char::from_str(name)
}

/// Represents an unrecognized header key‑value pair with its associated comments.
#[derive(Debug, Clone)]
pub struct ExtraHeaderKey {
    /// The raw line content of the key and value.
    pub line: String,
    /// Comments attached to this extra key.
    pub comments: Vec<String>,
}
/// A line containing one or more tags and optional comments.
#[derive(Default, Debug, Clone)]
pub struct Tagline {
    /// Set of tags on this line.
    pub tags: OrderSet<String>,
    /// Comments associated with this tag line.
    pub comments: Vec<String>,
}

impl fmt::Display for Tagline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in &self.comments {
            writeln!(f, ";; {}", c)?;
        }
        let mut linelen = 0;
        for (n, tag) in self.tags.iter().enumerate() {
            let tlen = tag.len() + 2;
            if n + 1 < self.tags.len() && linelen + tlen < 80 {
                write!(f, "#{} ", tag)?;
                linelen += tlen;
            } else {
                writeln!(f, "#{}", tag)?;
                linelen = 0;
            }
        }
        Ok(())
    }
}

impl FromStr for Tagline {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut tagline = Self::default();
        for tag in s.split(" ") {
            if let Some(tag) = tag.strip_prefix("#") {
                tagline.tags.insert(tag.into());
            }
        }
        Ok(tagline)
    }
}

/// Legacy color mode
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LegacyColorMode {
    /// No colors used.
    None,
    /// Only foreground colors used.
    FgOnly,
    /// Only background colors used.
    BgOnly,
    /// Both foreground and background colors used.
    FgAndBg,
}

impl Default for LegacyColorMode {
    fn default() -> Self {
        Self::None
    }
}

/// Legacy header information for backward compatibility.
#[derive(Default, Debug, Clone, Copy)]
pub struct LegacyHeaderInfo {
    pub colors: LegacyColorMode,
    pub width: usize,
    pub height: usize,
}

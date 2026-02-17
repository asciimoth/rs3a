use crate::error::{Error, Result};
use std::{fmt::Display, str::FromStr};
use std::convert::TryFrom;

/// Space character.
pub const SPACE: Char = Char { char: ' ' };
/// Underscore character.
pub const UNDERSCORE: Char = Char { char: '_' };

/// A validated character for use in 3a art.
/// Only allowed characters (printable, non‑control, etc.) can be contained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Char {
    pub(crate) char: char,
}

impl Char {
    /// Creates a new `Char` after validating the character.
    /// Returns `Err` if the character is not allowed.
    pub fn new(ch: char) -> Result<Self> {
        check_char(ch).map_or(Err(Error::DisallowedChar(ch.into())), |ok| {
            Ok(Self { char: ok })
        })
    }

    /// Creates a new `Char`; panics if the character is disallowed.
    /// Prefer `new` or `new_or` for safe construction.
    pub fn new_must(ch: char) -> Char {
        Self::new(ch).unwrap()
    }

    /// Creates a new `Char` if the character is allowed; otherwise returns the default.
    pub fn new_or(ch: char, default: Char) -> Char {
        check_char(ch).map_or(default, |ok| Char { char: ok })
    }
}

/// Formats Char as a single character.
impl Display for Char {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.char)
    }
}

/// Formats Char as a single character.
impl Into<char> for Char {
    fn into(self) -> char {
        self.char
    }
}

/// Formats Char as a single character.
impl Into<char> for &Char {
    fn into(self) -> char {
        self.char
    }
}

/// Allows conversion to the Unicode code point (u32).
impl Into<u32> for Char {
    fn into(self) -> u32 {
        self.char.into()
    }
}

/// Allows conversion to the Unicode code point (u32).
impl Into<u32> for &Char {
    fn into(self) -> u32 {
        self.char.into()
    }
}

/// Converts the character to a one‑character `String` without consuming.
impl Into<String> for Char {
    fn into(self) -> String {
        format!("{}", self)
    }
}

/// Converts the character to a one‑character `String` without consuming.
impl Into<String> for &Char {
    fn into(self) -> String {
        format!("{}", self)
    }
}

/// Parses a string slice containing exactly one character into a `Char`.
impl FromStr for Char {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() == 1 {
            Self::new(chars[0])
        } else {
            Err(Error::StrToCharConversion(chars.len()))
        }
    }
}

/// Equivalent to `Char::from_str`.
impl TryFrom<&str> for Char {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self> {
        Self::from_str(value)
    }
}

/// Equivalent to `Char::from_str`.
impl TryFrom<String> for Char {
    type Error = Error;
    fn try_from(value: String) -> Result<Self> {
        Self::from_str(&value)
    }
}

/// Checks whether a character is allowed in 3a art.
/// Returns `Some(ch)` if allowed (with some whitespace normalized to space),
/// or `None` if the character should be rejected.
pub fn check_char(ch: char) -> Option<char> {
    let cp = ch as u32;

    if ch == ' ' {
        return Some(' ');
    }

    // TAB U+0009
    if cp == 0x0009 {
        return Some(' ');
    }
    // Mongolian Vowel Separator U+180E (explicit)
    if cp == 0x180E {
        return Some(' ');
    }
    // Unicode "Space Separator" (Zs) set:
    // U+0020, U+00A0, U+1680, U+2000..U+200A, U+202F, U+205F, U+3000
    if cp == 0x0020
        || cp == 0x00A0
        || cp == 0x1680
        || (0x2000..=0x200A).contains(&cp)
        || cp == 0x202F
        || cp == 0x205F
        || cp == 0x3000
    {
        return Some(' ');
    }

    // C0 controls U+0000..U+001F
    if (0x0000..=0x001F).contains(&cp) {
        return None;
    }
    if [0x7F, 0x81, 0x8D, 0x8F, 0x90, 0x9D, 0xA0].contains(&cp) {
        return None;
    }
    // Combining marks U+0300..U+036F
    if (0x0300..=0x036F).contains(&cp) {
        return None;
    }
    // Zero-width / joiner: U+200B..U+200F, U+FEFF, U+FE00..U+FE0F
    if (0x200B..=0x200F).contains(&cp) || cp == 0xFEFF || (0xFE00..=0xFE0F).contains(&cp) {
        return None;
    }
    // Bidirectional control codes: U+202A..U+202E, U+2066..U+2069
    if (0x202A..=0x202E).contains(&cp) || (0x2066..=0x2069).contains(&cp) {
        return None;
    }
    // Surrogate code points U+D800..U+DFFF (defensive; won't appear in valid &str)
    if (0xD800..=0xDFFF).contains(&cp) {
        return None;
    }

    Some(ch)
}

/// Removes disallowed characters from a string and normalizes allowed whitespace.
/// The result contains only characters that would pass `check_char`.
pub fn normalize_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());

    for ch in input.chars() {
        if let Some(ch) = check_char(ch) {
            out.push(ch);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_cr_and_leaves_newline() {
        let s = "line1\r\nline2\nline3\r";
        let got = normalize_text(s);
        // CR removed, newlines preserved
        assert_eq!(got, "line1line2line3");
    }

    #[test]
    fn tabs_and_spaces_normalized() {
        let s = "A\tB\u{00A0}C\u{2003}D\u{205F}E\u{3000}F\u{180E}G";
        // tab, NBSP, EM SPACE (2003), MEDIUM MATHEMATICAL SPACE (205F), IDEOGRAPHIC SPACE (3000),
        // Mongolian Vowel Separator (180E) should all become ASCII spaces
        let got = normalize_text(s);
        assert_eq!(got, "A B C D E F G");
    }

    #[test]
    fn removes_zero_width_and_bidi_and_combining() {
        // zero width U+200B, bidi U+202A, combining acute accent U+0301
        let s = format!("X\u{200B}Y\u{202A}Z\u{0301}Q");
        let got = normalize_text(&s);
        // U+200B removed, U+202A removed, U+0301 removed (combining)
        assert_eq!(got, "XYZQ");
    }

    #[test]
    fn removes_control_c1_and_c0_except_newline_and_tab() {
        // U+0001 and U+0080 should be removed; newline and tab handled specially
        let s = format!("A\u{0001}B\u{001b}CD\tE");
        let got = normalize_text(&s);
        assert_eq!(got, "ABCD E");
    }

    #[test]
    fn preserves_other_unicode() {
        let s = "Привет\u{00A0}мир — hello";
        let got = normalize_text(s);
        // NBSP becomes ASCII space, other characters preserved
        assert_eq!(got, "Привет мир — hello");
    }

    #[test]
    fn example_from_doc() {
        let s = "Hello\u{00A0}World\t!\r\nA\u{200B}B\u{0301}C";
        let out = normalize_text(s);
        assert_eq!(out, "Hello World !ABC");
    }
}

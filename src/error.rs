use core::fmt::Display;
use std::{num::ParseIntError, sync::Arc};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    DelayLineParsing(String),
    DelayLineVoid(String),
    GlobalDelayParsing(String, ParseIntError),
    GlobalDelayDup(String),
    PerFrameDelayParsing(String, ParseIntError),
    PerFrameDelayDup(usize, String),

    ColorParsing(String),

    ColorDuplicate(String, String),

    HeaderKeyWithoutValue(String),
    HeaderKeyDup(String),
    HeaderFlagKey(String),

    PreviewParsing(String, ParseIntError),

    ColorName(String),
    ColorMapDup(String),

    WidthMismatch,
    HeightMismatch,
    FramesMismatch,

    ColorsMismatch,

    VoidTextChannel,

    BlockDup(String),

    BlockExpected(String),

    DisallowedChar(u32),
    StrToCharConversion(usize),

    Io(Arc<std::io::Error>),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.into())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DelayLineParsing(s) => write!(f, "failed to parse delay line: {}", s),
            Error::GlobalDelayDup(s) => {
                write!(f, "global delay presented multiple times in: {}", s)
            }
            Error::PerFrameDelayDup(fr, s) => {
                write!(
                    f,
                    "delay for frame {} presented multiple times in: {}",
                    fr, s
                )
            }
            Error::GlobalDelayParsing(s, err) => {
                write!(f, "fail to parse global delay '{}' :{}", s, err)
            }
            Error::DelayLineVoid(s) => write!(f, "no delay values foind in: {}", s),
            Error::PerFrameDelayParsing(s, err) => {
                write!(f, "fail to parse per-frame delay '{}' :{}", s, err)
            }
            Error::ColorParsing(s) => write!(f, "failed to parse color: {}", s),
            Error::ColorDuplicate(n, l) => {
                write!(f, "{} duplicates in: {}", n, l)
            }
            Error::Io(err) => err.fmt(f),
            Error::HeaderKeyWithoutValue(k) => write!(f, "header key '{}' have no value", k),
            Error::HeaderKeyDup(k) => write!(f, "header key '{}' duplicates", k),
            Error::HeaderFlagKey(k) => write!(
                f,
                "failed to parse header flag key '{}'; value must be 'yes' or 'no'",
                k
            ),
            Error::PreviewParsing(v, err) => {
                write!(f, "failed to parse preview value '{}': {}", v, err,)
            }
            Error::ColorName(name) => write!(f, "'{}' cannot be used as color name", name),
            Error::ColorMapDup(name) => write!(f, "color mapping for '{}' duplicates", name),
            Error::WidthMismatch => {
                write!(f, "width of some art components do not match each other")
            }
            Error::HeightMismatch => {
                write!(f, "height of some art components do not match each other")
            }
            Error::BlockDup(name) => write!(f, "block {} duplicated", name),
            Error::BlockExpected(line) => write!(f, "block title expected, got: {}", line),
            Error::FramesMismatch => write!(f, "channels frame count mismatch"),
            Error::ColorsMismatch => write!(f, "color info from header and body mismatch"),
            Error::VoidTextChannel => write!(f, "0 frames in text channel"),
            Error::DisallowedChar(ch) => write!(f, "disallowed char witch code: {}", ch),
            Error::StrToCharConversion(ln) => {
                write!(f, "cannot convert str with length {} to single Char", ln)
            }
        }
    }
}

impl std::error::Error for Error {}

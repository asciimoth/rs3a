//! A library for working with the "3a" animated ASCII art format.
//!
//! Provides functionality to:
//! - Read and write 3a files (including legacy deprecated format).
//! - Convert animations to animated SVG or asciicast v2 format.
//! - Edit art programmatically via a comprehensive API.
//!
//! The core type is [`Art`], which represents a complete animation with
//! header, frames, and optional metadata. Frames consist of a grid of
//! cells, each containing a character and optional color mapping.

pub mod art;
pub mod chars;
pub mod colors;
pub mod comments;
pub mod content;
pub mod delay;
pub mod error;
pub mod font;
pub mod header;
mod helpers;

pub use art::Art;
pub use colors::{CSSColorMap, Color, Color4, ColorPair, Palette};
pub use comments::Comments;
pub use content::{Cell, Frame, Frames};
pub use delay::Delay;
pub use error::{Error, Result};
pub use header::{ExtraHeaderKey, Header, LegacyColorMode, LegacyHeaderInfo, Tagline};

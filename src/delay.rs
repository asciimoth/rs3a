use core::fmt;
use std::{collections::HashMap, str::FromStr};

use crate::error::{Error, Result};

/// Frame delay configuration for animations.
/// Contains a global delay and optional per-frame overrides.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Delay {
    /// Global delay in milliseconds, applied to all frames unless overridden.
    /// A value of 0 is interpreted as the default (50ms).
    pub global: usize,
    /// Per-frame delay overrides, keyed by frame index (0-based).
    pub per_frame: HashMap<usize, usize>,
}

impl Delay {
    /// Returns the effective global delay, defaulting to 50ms if set to 0.
    pub fn get_global(&self) -> usize {
        if self.global == 0 { 50 } else { self.global }
    }
    /// Returns the effective delay for a specific frame, falling back to global.
    pub fn get_frame(&self, frame: usize) -> usize {
        let d = self.per_frame.get(&frame).unwrap_or(&self.global).clone();
        if d == 0 { 50 } else { d }
    }
    /// Sets the global delay. If `global` is 0, it is interpreted as the default (50ms).
    pub fn set_global(&mut self, global: usize) {
        self.global = if global == 0 { 50 } else { global };
    }
    /// Sets a per-frame delay override. If `delay` is 0, the override is removed.
    pub fn set_frame(&mut self, frame: usize, delay: usize) {
        if delay == 0 {
            self.per_frame.remove(&frame);
        } else {
            self.per_frame.insert(frame, delay);
        }
    }
    /// Optimizes the delay map after changing the total frame count.
    /// - Removes overrides for frames beyond `count`.
    /// - If all remaining frames have the same delay, promotes it to global.
    /// - Removes overrides that equal the new global delay.
    pub fn set_frames(&mut self, count: usize) {
        let mut global = self.get_global();
        let mut per_frame = HashMap::<usize, usize>::new();
        let mut last: usize = 0;
        let mut diff = false;
        for (frame, delay) in &self.per_frame {
            if *frame < count {
                if last != 0 && last != *delay {
                    diff = true;
                }
                last = *delay;
                if *delay != global {
                    per_frame.insert(*frame, *delay);
                }
            }
        }
        if last != 0 && !diff && per_frame.len() == count {
            global = last;
            per_frame = HashMap::new();
        }
        self.global = global;
        self.per_frame = per_frame;
    }
    /// Returns a vector of delays for all frames from 0 to `frames-1`.
    /// Each entry is the effective delay for that frame.
    pub fn to_vec_delays(&self, frames: usize) -> Vec<usize> {
        let mut delays = vec![];
        for f in 0..frames {
            delays.push(self.get_frame(f));
        }
        delays
    }
}

/// Formats the delay as a string: global value followed by space-separated
/// "frame:delay" pairs.
impl fmt::Display for Delay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_global())?;
        let mut per_frame = self.per_frame.iter().collect::<Vec<_>>();
        per_frame.sort_by_key(|p| p.0.clone());
        for (frame, delay) in per_frame {
            write!(f, " {}:{}", frame, delay)?;
        }
        Ok(())
    }
}

/// Parses a delay string of the form "global [frame:delay ...]".
/// Returns an error if the format is invalid or duplicates exist.
impl FromStr for Delay {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut ret = Self {
            global: 0,
            per_frame: HashMap::new(),
        };
        let mut delays = 0;
        for ss in s.trim().split(" ") {
            if s.is_empty() {
                continue;
            }
            delays += 1;
            match ss.split_once(":") {
                Some((f, d)) => {
                    let f = match f.parse::<usize>() {
                        Ok(f) => f,
                        Err(err) => {
                            return Err(Error::PerFrameDelayParsing(String::from(ss), err));
                        }
                    };
                    let d = match d.parse::<usize>() {
                        Ok(d) => d,
                        Err(err) => {
                            return Err(Error::PerFrameDelayParsing(String::from(ss), err));
                        }
                    };
                    if ret.per_frame.contains_key(&f) {
                        return Err(Error::PerFrameDelayDup(f, String::from(ss)));
                    }
                    ret.per_frame.insert(f, d);
                }
                None => match ss.parse::<usize>() {
                    Ok(g) => ret.global = g,
                    Err(err) => {
                        return Err(Error::GlobalDelayParsing(String::from(ss), err));
                    }
                },
            };
        }
        if delays == 0 {
            return Err(Error::DelayLineVoid(String::from(s)));
        }
        if ret.global == 0 {
            ret.global = 50
        }
        Ok(ret)
    }
}

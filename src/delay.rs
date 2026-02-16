use core::fmt;
use std::{collections::HashMap, str::FromStr};

use crate::error::{Error, Result};

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Delay {
    pub global: usize,
    pub per_frame: HashMap<usize, usize>,
}

impl Delay {
    pub fn to_vec_delays(&self, frames: usize) -> Vec<usize> {
        let mut delays = vec![];
        for f in 0..frames {
            delays.push(self.get_frame(f));
        }
        delays
    }
    pub fn get_global(&self) -> usize {
        if self.global == 0 { 50 } else { self.global }
    }
    pub fn get_frame(&self, frame: usize) -> usize {
        let d = self.per_frame.get(&frame).unwrap_or(&self.global).clone();
        if d == 0 { 50 } else { d }
    }
    // Use global == 0 to set global delay to default one (50 milis)
    pub fn set_global(&mut self, global: usize) {
        self.global = if global == 0 { 50 } else { global };
    }
    // Use zero delay to remove value
    pub fn set_frame(&mut self, frame: usize, delay: usize) {
        if delay == 0 {
            self.per_frame.remove(&frame);
        } else {
            self.per_frame.insert(frame, delay);
        }
    }
    // Remove all per-frame values for frames >= count
    // If there is a same per frame value for all frames < count, set global to it
    // Remove per-frame values eqalled to global
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
}

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

use std::convert::TryInto;
use std::io::Read;

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtyrecFrame {
    pub(crate) timestamp_ms: usize,
    /// Raw terminal data (e.g., escape sequences, text).
    pub(crate) text: String,
}

impl TtyrecFrame {
    pub(crate) fn append_to_vec(&self, output: &mut Vec<u8>) {
        let sec = (self.timestamp_ms / 1000) as u32;
        let usec = ((self.timestamp_ms % 1000) * 1000) as u32;
        let data: Vec<_> = self.text.bytes().collect();
        let len = data.len() as u32;

        output.extend_from_slice(&sec.to_le_bytes());
        output.extend_from_slice(&usec.to_le_bytes());
        output.extend_from_slice(&len.to_le_bytes());
        output.extend_from_slice(&data);
    }
}

pub(crate) struct TtyrecReader<R: Read> {
    reader: R,
    buf: Vec<u8>,
    eof: bool,
}

impl<R: Read> TtyrecReader<R> {
    pub(crate) fn new(reader: R) -> Self {
        TtyrecReader {
            reader,
            buf: Vec::new(),
            eof: false,
        }
    }

    pub(crate) fn next_frame(&mut self) -> Result<Option<TtyrecFrame>, Error> {
        if self.eof {
            return Ok(None);
        }

        // Read header: 12 bytes (three u32 little‑endian values)
        let mut header = [0u8; 12];
        match self.reader.read_exact(&mut header) {
            Ok(()) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                self.eof = true;
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        }

        let sec = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let usec = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let len = u32::from_le_bytes(header[8..12].try_into().unwrap());

        // Read payload
        self.buf.resize(len as usize, 0);
        self.reader.read_exact(&mut self.buf)?;

        // Convert timestamp to milliseconds with rounding
        let total_ms = (sec as u64) * 1000 + ((usec as u64) + 500) / 1000;

        // Cast to usize; return overflow error if it doesn't fit
        let timestamp_ms = total_ms.try_into().map_err(|_| Error::DelayOverflow)?;

        Ok(Some(TtyrecFrame {
            timestamp_ms,
            text: String::from_utf8(self.buf.clone()).map_err(|_| Error::NotUtf8)?,
        }))
    }
}

impl<R: Read> Iterator for TtyrecReader<R> {
    type Item = Result<TtyrecFrame, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_frame() {
            Ok(Some(frame)) => Some(Ok(frame)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

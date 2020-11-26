//! A Seekable `Take` implementation.

#![cfg_attr(not(debug_assertions), deny(warnings, missing_docs, clippy::dbg_macro))]

#![cfg_attr(feature = "read_initializer", feature(read_initializer))]

use std::cmp;
use std::io::{Error,ErrorKind,Read, Seek, SeekFrom, Result};

/// Extension trait for `Read + Seek` to support `takes`
pub trait Ext : Read + Seek + Sized {
    /// Returns a seekable Take.
    ///
    /// # Errors
    /// Returns an error if the current offset could not be seeked.
    fn takes(self, limit: u64) -> Result<Takes<Self>>;
}

impl<R: Read + Seek> Ext for R {
    fn takes(mut self, limit: u64) -> Result<Takes<Self>> {
        let start = self.seek(SeekFrom::Current(0))?;

        Ok(Takes {
            inner: self,
            start,
            limit,
            current: 0,
        })
    }
}

/// A Seekable `Take` implementation.
///
/// Note that Seek offsets may not start at zero.
pub struct Takes<R> {
    inner: R,
    start: u64,
    limit: u64,
    current: u64, // number of bytes of current pointer from start
}

// Code based on std::io::Take implementation
impl<R: Read> Read for Takes<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let rem = self.limit - self.current;
        // Don't call into inner reader at all at EOF because it may still block
        if rem == 0 {
            return Ok(0);
        }

        let max = cmp::min(buf.len() as u64, rem) as usize;
        let n = self.inner.read(&mut buf[..max])?;
        self.current += n as u64;
        Ok(n)
    }

    #[cfg(feature = "read_initializer")]
    unsafe fn initializer(&self) -> Initializer {
        self.inner.initializer()
    }
}

/// The absolute offsets used in the Seek implementation are *identical* to those in the underlying
/// Read.
/// In other words, `SeekFrom::Start(0)` may seek beyond range and cause error.
impl<R: Seek> Seek for Takes<R> {
    fn seek(&mut self, seek: SeekFrom) -> Result<u64> {
        Ok(match seek {
            SeekFrom::Start(offset) => {
                if offset < self.start || offset > self.current {
                    return Err(Error::new(ErrorKind::UnexpectedEof, "cannot seek beyond Takes range"));
                }
                self.inner.seek(SeekFrom::Start(offset))?
            },
            SeekFrom::Current(delta) => {
                let dest = (self.current as i64) + delta;
                if dest < 0 || (dest as u64) > self.limit {
                    return Err(Error::new(ErrorKind::UnexpectedEof, "cannot seek beyond Takes range"));
                }
                self.inner.seek(SeekFrom::Current(delta))?
            },
            SeekFrom::End(_) => unimplemented!("SeekFrom::End implementation would be ambiguous"),
        })
    }
}

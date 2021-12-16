use std::cell;
use std::io;
use std::io::Read;
use std::str;

use log::*;

use crate::archive::ArchiveError;
use crate::checksum::*;

const HEADER_SIZE: usize = 110;
const MAGIC_NUMBER: &[u8] = b"070701";
const TRAILER: &str = "TRAILER!!!";

/// A wrapper around io::Read which counts the number of bytes read.
#[derive(Debug)]
struct PosReader<R: io::Read> {
    pub count: usize,
    inner: R,
}

impl<R: io::Read> io::Read for PosReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        trace!("reading pos: {}", self.count);
        let count = self.inner.read(buf)?;
        self.count += count;
        Ok(count)
    }
}

//#[derive(Debug)]
pub struct CpioFile<'a, R: io::Read> {
    pub filename: String,
    pub filesize: u32,
    remaining: usize,
    reader: &'a cell::RefCell<PosReader<R>>,
    cksum: Checksum,
}

impl<'a, R: io::Read> io::Read for CpioFile<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut reader = self.reader.borrow_mut();

        //trace!("remaining: {}", self.remaining);
        // maximum to read is the end of the contained file
        let max_read = usize::min(buf.len(), self.remaining);
        let bytes_read = reader.read(&mut buf[0..max_read])?;
        self.remaining -= bytes_read;

        // update the running checksum
        self.cksum.update(&buf[0..bytes_read]);

        Ok(bytes_read)
    }
}

impl<'a, R: io::Read> CpioFile<'a, R> {
    pub fn finalise(&mut self, cksum_expected: Checksum) -> Result<(), ArchiveError> {
        assert_eq!(self.remaining, 0);

        self.cksum.finalise();
        if self.cksum != cksum_expected {
            return Err(ArchiveError::ChecksumMismatchError { filename: self.filename.clone() });
        }
        Ok(())
    }
}

pub struct CpioReader<R: io::Read> {
    reader: cell::RefCell<PosReader<R>>,
}

impl<'a, R: io::Read> CpioReader<R> {
    pub fn new(reader: R) -> CpioReader<R> {
        CpioReader {
            reader: cell::RefCell::new(PosReader {
                count: 0,
                inner: reader,
            }),
        }
    }

    fn read_hex_u32(reader: &mut cell::RefMut<PosReader<R>>) -> Result<u32, ArchiveError> {
        let mut buf = [0u8; 8];
        if let Err(err) = reader.read_exact(&mut buf) {
            return Err(ArchiveError::IOError { source: err });
        }
        let hexstr = str::from_utf8(&buf).map_err(|err| ArchiveError::ParseError(Box::new(err)))?;
        let val = u32::from_str_radix(hexstr, 16)
            .map_err(|err| ArchiveError::ParseError(Box::new(err)))?;
        Ok(val)
    }

    pub fn read_next_file(&'a self) -> Result<Option<CpioFile<'a, R>>, ArchiveError> {
        // the previous file needs to be completely read before we get here or we'll fail
        //  if this is not the case, the cpio header checks should fail
        let mut reader = self.reader.borrow_mut();

        if reader.count > 0 {
            let trailing = (4 - (reader.count % 4)) % 4;
            //trace!("reading {} more bytes", trailing);
            let mut trailing_buf = [0u8; 4];
            reader.read_exact(&mut trailing_buf[0..trailing as usize])?;
        }

        let mut buf = [0u8; 256];
        {
            let mut buf = &mut buf[0..MAGIC_NUMBER.len()];
            if let Err(err) = io::Read::read_exact(&mut *reader, &mut buf) {
                return Err(ArchiveError::IOError { source: err });
            }
            debug!("magic: {}", str::from_utf8(&buf[..buf.len()]).unwrap());
            if buf != MAGIC_NUMBER {
                return Err(ArchiveError::FormatError {
                    offset: reader.count,
                    reason: "magic number mismatch".to_owned(),
                });
            }
        }

        Self::read_hex_u32(&mut reader)?; //ino
        Self::read_hex_u32(&mut reader)?; //mode
        Self::read_hex_u32(&mut reader)?; //uid
        Self::read_hex_u32(&mut reader)?; //gid
        Self::read_hex_u32(&mut reader)?; //nlink
        Self::read_hex_u32(&mut reader)?; //mtime
        let filesize = Self::read_hex_u32(&mut reader)?;
        Self::read_hex_u32(&mut reader)?; //dev-major
        Self::read_hex_u32(&mut reader)?; //dev-minor
        Self::read_hex_u32(&mut reader)?; //rdev-major
        Self::read_hex_u32(&mut reader)?; //rdev-minor
        let namesize = Self::read_hex_u32(&mut reader)?;
        let check = Self::read_hex_u32(&mut reader)?;

        if check != 0 {
            return Err(ArchiveError::FormatError {
                offset: reader.count,
                reason: "check field non-zero".to_owned(),
            });
        }

        // this isn't a hard limit on cpio format, but we really shouldn't need filenames longer
        // than this.
        if namesize as usize > buf.len() {
            return Err(ArchiveError::FormatError {
                offset: reader.count,
                reason: format!("unexpectedly long filename size: {}", namesize).to_owned(),
            });
        }

        let mut buf = &mut buf[0..namesize as usize];
        if let Err(err) = reader.read_exact(&mut buf) {
            return Err(ArchiveError::IOError { source: err });
        }
        let filename = str::from_utf8(&buf[..(buf.len() - 1)])
            .map_err(|err| ArchiveError::ParseError(Box::new(err)))?;
        debug!("filename: {}", filename);

        // the size of the header is rounded up to the next 4-byte boundary
        {
            let bytes_read = HEADER_SIZE + namesize as usize;
            let mut trailing_buf = [0u8; 4];
            let trailing = (4 - (bytes_read % 4)) % 4;
            //trace!("trailing: {}", trailing);
            reader.read_exact(&mut trailing_buf[0..trailing])?;
        }

        if filename == TRAILER {
            // trailer filename indicates end of archive
            return Ok(None);
        }

        let mut cpio_file = CpioFile {
            filesize,
            remaining: filesize as usize,
            filename: String::from(filename),
            reader: &self.reader,
            cksum: Checksum::new_hashable(),
        };
        cpio_file.remaining = cpio_file.filesize as usize;

        Ok(Some(cpio_file))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::*;
    use std::fs;
    use std::io::Read;

    #[test]
    fn empty() {
        init_logging();
        let path = test_path("cpio/empty.cpio");

        let mut file = fs::File::open(path).unwrap();
        let reader = CpioReader::new(&mut file);
        if let Err(err) = reader.read_next_file() {
            assert!(matches!(err, ArchiveError::IOError { .. }));
        } else {
            panic!("expected empty archive to error");
        }
    }

    #[test]
    fn two_files() {
        init_logging();
        let path = test_path("cpio/two-files.cpio");

        let file = fs::File::open(path).unwrap();
        let reader = CpioReader::new(file);

        let mut nfile = 0;
        loop {
            match reader.read_next_file() {
                Ok(next_file) => {
                    match next_file {
                        Some(mut file) => {
                            nfile += 1;
                            let mut buf = [0u8; 32];
                            let bytes_read = file.read(&mut buf).unwrap();

                            if bytes_read == 0 {
                                // eof
                                continue;
                            }

                            if nfile == 1 {
                                assert_eq!(file.filesize, 6);
                                assert_eq!(str::from_utf8(&buf[0..bytes_read]).unwrap(), "data!\n");
                            } else if nfile == 2 {
                                assert_eq!(
                                    str::from_utf8(&buf[0..bytes_read]).unwrap(),
                                    "more-data\n"
                                );
                            } else {
                                panic!("extra unexpected file");
                            }
                        }
                        None => break,
                    }
                }
                Err(err) => panic!("panic on next file: {}", err),
            }
        }
    }
}

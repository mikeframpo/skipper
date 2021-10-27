use std::cell;
use std::error;
use std::fmt;
use std::io;
use std::str;

const HEADER_SIZE: usize = 110;
const MAGIC_NUMBER: &[u8] = b"070701";
const TRAILER: &str = "TRAILER!!!";

#[derive(Debug)]
struct CpioFile<'a, R: io::Read> {
    filesize: u32,
    remaining: usize,
    filename: String,
    reader: &'a cell::RefCell<R>
}

impl<'a, R: io::Read> io::Read for CpioFile<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut reader = self.reader.borrow_mut();
        let max_read = usize::min(buf.len(), self.remaining);
        let bytes_read = reader.read(&mut buf[0..max_read])?;
        self.remaining -= bytes_read;

        if self.remaining == 0 {
            // data is rounded up to the next 4-byte boundary
            let trailing = 4 - (self.filesize % 4);
            let mut trailing_buf = [0u8; 4];
            reader
                .read_exact(&mut trailing_buf[0..trailing as usize])?;
        }

        Ok(bytes_read)
    }
}

#[derive(Debug)]
pub enum CpioError {
    /// Represents a mis-formatted input file, or failure to parse an element within the file.
    FormatError(String),
    /// Represents an error while reading from the input stream.
    IOError(io::Error),
}

impl fmt::Display for CpioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpioError::FormatError(str) => write!(f, "format error: {}", str),
            CpioError::IOError(_) => write!(f, "io error"),
        }
    }
}

impl error::Error for CpioError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            CpioError::FormatError(_) => None,
            CpioError::IOError(ref source) => Some(source),
        }
    }
}

pub struct CpioReader<R: io::Read> {
    reader: cell::RefCell<R>,
}

impl<'a, R: io::Read> CpioReader<R> {
    pub fn new(reader: R) -> CpioReader<R> {
        CpioReader {
            reader: cell::RefCell::new(reader),
        }
    }

    fn read_hex_u32(reader: &mut cell::RefMut<R>) -> Result<u32, CpioError> {
        let mut buf = [0u8; 8];
        if let Err(err) = reader.read_exact(&mut buf) {
            return Err(CpioError::IOError(err));
        }
        // TODO: wrap inner errors in cpioerror
        let hexstr = str::from_utf8(&buf)
            .map_err(|_| CpioError::FormatError(String::from("hexstr read error")))?;
        let val = u32::from_str_radix(hexstr, 16)
            .map_err(|_| CpioError::FormatError(String::from("hex parse error")))?;
        Ok(val)
    }

    fn read_next_file(&'a self) -> Result<Option<CpioFile<'a, R>>, CpioError> {
        let mut reader = self.reader.borrow_mut();
        let mut buf = [0u8; 256];
        {
            let mut buf = &mut buf[0..MAGIC_NUMBER.len()];
            if let Err(err) = reader.read_exact(&mut buf) {
                return Err(CpioError::IOError(err));
            }
            if buf != MAGIC_NUMBER {
                // TODO: need an error message on here
                return Err(CpioError::FormatError(String::from(
                    "magic number mismatch",
                )));
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
            return Err(CpioError::FormatError(String::from(
                "check field was non-zero",
            )));
        }

        // this isn't a hard limit on cpio format, but we really shouldn't need filenames longer
        // than this.
        if namesize as usize > buf.len() {
            return Err(CpioError::FormatError(String::from(
                "unexpectedly long filename",
            )));
        }

        let mut buf = &mut buf[0..namesize as usize];
        if let Err(err) = reader.read_exact(&mut buf) {
            return Err(CpioError::IOError(err));
        }
        let filename = str::from_utf8(&buf[..(buf.len()-1)])
            .map_err(|_| CpioError::FormatError(String::from("failed to read filename")))?;

        // the size of the header is rounded up to the next 4-byte boundary
        {
            let bytes_read = HEADER_SIZE + namesize as usize;
            let mut trailing_buf = [0u8; 4];
            let trailing = 4 - (bytes_read % 4);
            reader
                .read_exact(&mut trailing_buf[0..trailing])
                .map_err(|err| CpioError::IOError(err))?;
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
        };
        cpio_file.remaining = cpio_file.filesize as usize;

        Ok(Some(cpio_file))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use std::io::Read;
    use std::path;

    #[test]
    fn empty() {
        let mut path = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test/cpio/empty.cpio");

        let mut file = fs::File::open(path).unwrap();
        let reader = CpioReader::new(&mut file);
        let err = reader.read_next_file().unwrap_err();
        //assert!(matches!(err, CpioError::IOError {..}));
        match err {
            CpioError::IOError(inner) => println!("Error while reading: {}", inner),
            _ => panic!(),
        }
    }

    #[test]
    fn two_files() {
        let mut path = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test/cpio/two-files.cpio");

        let file = fs::File::open(path).unwrap();
        let reader = CpioReader::new( file);

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

use std::io::Read;
use std::{error, io};
use log::{debug};
use thiserror::Error;

use crate::cpio::CpioReader;
use crate::payload;
use crate::payload::Payload;

pub struct Manifest;

pub struct Archive<R: io::Read> {
    cpio_reader: CpioReader<R>,
    manifest: Manifest,
}

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("archive: io error")]
    IOError {
        #[from]
        source: io::Error,
    },

    #[error("archive: parse error")]
    ParseError(Box<dyn error::Error>),

    #[error("archive: format error in field: {} at offset: {}", reason, offset)]
    FormatError { offset: usize, reason: String },

    #[error("manifest not found")]
    ManifestError,
}

#[derive(Error, Debug)]
#[error("archive: manifest not found")]
pub struct ManifestNotFoundError;

impl<'a, R: io::Read> Archive<R> {
    fn new(reader: R) -> Archive<R> {
        let cpio_reader = CpioReader::new(reader);
        // TODO: return Result
        let manifest = read_manifest(&cpio_reader).unwrap();
        Archive {
            cpio_reader,
            manifest,
        }
    }

    fn get_next_payload(&'a self) -> Result<Option<Box<dyn Payload + 'a>>, ArchiveError> {
        let next_file = self.cpio_reader.read_next_file()?;
        Ok(next_file.map(|next| {
            // TODO: implement a real payload handler
            let payload = payload::test::TestPayload { reader: next };
            Box::new(payload) as Box<dyn Payload>
        }))
    }
}

fn read_manifest<R: io::Read>(cpio_reader: &CpioReader<R>) -> Result<Manifest, ArchiveError> {
    let manifest_file = cpio_reader.read_next_file()?;
    manifest_file
        .ok_or(ArchiveError::ManifestError)
        .map(|mut file| {
            let mut burn_buf = vec![0u8; 1024];
            let num_bytes = file.read(&mut burn_buf).unwrap();
            debug!("manifest read {} bytes", num_bytes);
            let data = std::str::from_utf8(&burn_buf[..num_bytes]).unwrap();
            debug!("got manifest data: {}", data);

            // TODO: need to read and parse manifest properly somewhere
            Manifest {}
        })
}

fn process_payload(_manifest: &Manifest, _payload: Box<dyn Payload>) {
    todo!()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{fs, path};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn basics() {
        init();
        let mut path = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test/archive/test.cpio");

        let input = fs::File::open(path).unwrap();
        let archive = Archive::new(input);

        let payload = archive.get_next_payload().unwrap();
        assert_eq!(payload.unwrap().deploy().unwrap(), 1024);
    }
}

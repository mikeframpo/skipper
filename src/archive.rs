use log::debug;
use std::io::Read;
use std::{error, io};
use thiserror::Error;

use crate::cpio::CpioReader;
use crate::manifest::{self, Manifest};
use crate::payload::Payload;

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
    NoManifestError,

    #[error("manifest parse error, cause: {0}")]
    ManifestParseError(serde_json::Error),

    #[error("archive: utf8 parse error, cause: {}", source)]
    Utf8Error {
        #[from]
        source: std::str::Utf8Error,
    },
}

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
        Ok(next_file.map(|_next| {
            // TODO: implement a real payload handler
            //let payload = payload::test::TestPayload { reader: next };
            //Box::new(payload) as Box<dyn Payload>
            todo!()
        }))
    }
}

fn read_manifest<R: io::Read>(cpio_reader: &CpioReader<R>) -> Result<Manifest, ArchiveError> {
    let manifest_file = cpio_reader.read_next_file()?;
    if manifest_file.is_none() {
        return Err(ArchiveError::NoManifestError);
    }

    let mut manifest_file = manifest_file.unwrap();
    if manifest_file.filename != "manifest.json" {
        return Err(ArchiveError::NoManifestError);
    }

    let mut burn_buf = vec![0u8; 4096];
    // TODO: need to make read_to_end work properly
    let count = manifest_file.read(&mut burn_buf)?;
    debug!("manifest read {} bytes", count);

    let data = std::str::from_utf8(&burn_buf[..count])?;
    debug!("manifest data: {}", data);
    let manifest =
        manifest::parse_manifest(data).map_err(|err| ArchiveError::ManifestParseError(err))?;
    Ok(manifest)
}

fn process_payload(_manifest: &Manifest, _payload: Box<dyn Payload>) {
    todo!()
}

#[cfg(test)]
mod test {
    use crate::payload::Status;

    use super::*;
    use crate::test_utils::*;
    use std::fs;

    #[test]
    fn basics() {
        // note that a new archive file can be generated with the following command
        //
        // $ echo -e "manifest.json\nimage-file" | cpio -ov --format=newc > test.cpio
        //
        init_logging();
        let path = test_path("archive/test.cpio");

        let input = fs::File::open(path).unwrap();
        let archive = Archive::new(input);

        let payload = archive.get_next_payload().unwrap();
        assert_eq!(payload.unwrap().deploy().unwrap(), Status::Complete);
    }
}

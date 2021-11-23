use std::{
    fs::File,
    io::{self, Write},
    path::{PathBuf},
};

use log::debug;

use crate::{archive::ArchiveError};

// Represents the disk-image, file, directory payload data to be written to disk.
pub trait Payload {
    fn deploy(&mut self) -> Result<Status, ArchiveError>;
}

#[derive(PartialEq, Debug)]
pub enum Status {
    Complete,
    Pending,
}

pub struct ImagePayload<R: io::Read> {
    reader: R,
    dest: PathBuf,
    dest_file: Option<File>,
}

impl<R: io::Read> ImagePayload<R> {
    pub fn new(reader: R, dest: PathBuf) -> ImagePayload<R> {
        ImagePayload {
            reader,
            dest,
            dest_file: None,
        }
    }

    fn open_dest_file(&mut self) -> Result<(), ArchiveError> {
        self.dest_file = Some(File::create(&self.dest)?);
        Ok(())
    }
}

impl<R: io::Read> Payload for ImagePayload<R> {
    fn deploy(&mut self) -> Result<Status, ArchiveError> {
        if self.dest_file.is_none() {
            self.open_dest_file()?;
        }
        let mut buf = vec![0u8; 2048];
        let read_count = self.reader.read(&mut buf)?;
        debug!("read {} bytes from reader", read_count);

        // TODO: need to implement multi-call deployments
        assert!(read_count < buf.len());

        self.dest_file
            .as_mut()
            .unwrap()
            .write_all(&buf[..read_count])?;
        debug!("wrote {} bytes to dest", read_count);
        Ok(Status::Complete)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::test_utils::*;
    use std::{path, process::Command};

    #[test]
    fn test_deploy_image() {
        init_logging();
        let path = test_path("archive/test.img");
        let img_file = File::open(path.clone()).unwrap();

        // TODO: should be able to generate a random test filename
        let dest_path = path::PathBuf::from("/tmp/imgdest");

        let mut payload = ImagePayload::new(img_file, dest_path.clone());
        assert_eq!(payload.deploy().unwrap(), Status::Complete);

        // use the cmp utility to compare the files
        assert!(
            Command::new("cmp")
                .arg(path)
                .arg(dest_path)
                .output()
                .unwrap()
                .status.success()
        );
    }
}

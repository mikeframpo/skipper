use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

use log::debug;

use crate::archive::ArchiveError;

// Represents the disk-image, file, directory payload data to be written to disk.
pub trait Payload {
    fn read_block(&mut self, buf: &mut [u8]) -> Result<usize, ArchiveError>;

    fn write_begin(&mut self) -> Result<(), ArchiveError>;

    fn write_block(&mut self, buf: &[u8]) -> Result<Status, ArchiveError>;

    fn deploy(&mut self) -> Result<(), ArchiveError> {
        self.write_begin()?;

        loop {
            let mut buf = vec![0u8; 2048];
            let read_count = self.read_block(&mut buf)?;

            if read_count == 0 {
                return Err(ArchiveError::PayloadDeployError {
                    reason: String::from("payload read completed before write finished"),
                });
            }

            let write_status = self.write_block(&buf[0..read_count])?;
            if write_status == Status::Complete {
                return Ok(());
            }
            // else { Status::Pending, keep reading }
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Status {
    Complete,
    Pending,
}

pub struct ImagePayload<R: io::Read> {
    reader: R,
    image_size: u64,
    remaining: u64,
    dest: PathBuf,
    dest_file: Option<File>,
}

impl<R: io::Read> ImagePayload<R> {
    pub fn new(reader: R, image_size: u64, dest: PathBuf) -> ImagePayload<R> {
        ImagePayload {
            reader,
            image_size,
            remaining: image_size,
            dest,
            dest_file: None,
        }
    }
}

impl<R: io::Read> Payload for ImagePayload<R> {
    fn read_block(&mut self, buf: &mut [u8]) -> Result<usize, ArchiveError> {
        let read_count = self.reader.read(buf)?;
        debug!("read {} bytes from reader", read_count);
        Ok(read_count)
    }

    fn write_begin(&mut self) -> Result<(), ArchiveError> {
        // open the destination file
        // TODO: I think this will fail for a block device
        self.dest_file = Some(File::create(&self.dest)?);
        debug!(
            "opened destination: {}",
            self.dest
                .to_str()
                .expect("destination file must be valid unicode")
        );
        Ok(())
    }

    fn write_block(&mut self, buf: &[u8]) -> Result<Status, ArchiveError> {
        // TODO: optionally deploy to dest on worker thread

        if self.remaining < buf.len() as u64 {
            return Err(ArchiveError::PayloadDeployError {
                reason: String::from("payload write overflow"),
            });
        }

        self.dest_file.as_mut().unwrap().write_all(buf)?;
        debug!("wrote {} bytes to dest", buf.len());

        self.remaining -= buf.len() as u64;
        if self.remaining == 0 {
            return Ok(Status::Complete);
        }
        Ok(Status::Pending)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::test_utils::*;
    use std::process::Command;

    fn do_image_test(image_path: &PathBuf) {
        let img_file = File::open(image_path.clone()).unwrap();
        let file_size = img_file.metadata().unwrap().len();

        let dest_path = make_tempfile_path();
        let mut payload = ImagePayload::new(img_file, file_size, dest_path.clone());
        assert_eq!(payload.deploy().unwrap(), ());

        // use the cmp utility to compare the files
        assert!(Command::new("cmp")
            .arg(image_path)
            .arg(dest_path)
            .output()
            .unwrap()
            .status
            .success());
    }

    #[test]
    fn test_deploy_image() {
        init_logging();
        let path = test_path("archive/test.img");
        do_image_test(&path);
    }

    #[test]
    fn test_deploy_larger_image() {
        init_logging();
        let path = test_path("archive/test-img-larger.img");
        do_image_test(&path);
    }
}

use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

use log::debug;

use crate::archive::ArchiveError;

// Represents the disk-image, file, directory payload data to be written to disk.
pub trait Payload {
    fn write_begin(&mut self) -> Result<(), ArchiveError>;

    fn write_block(&mut self, buf: &[u8]) -> Result<Status, ArchiveError>;
}

#[derive(PartialEq, Debug)]
pub enum Status {
    Complete,
    Pending,
}

pub struct ImagePayload {
    image_size: u64,
    remaining: u64,
    dest: PathBuf,
    dest_file: Option<File>,
}

impl ImagePayload {
    pub fn new(image_size: u64, dest: PathBuf) -> ImagePayload {
        ImagePayload {
            image_size,
            remaining: image_size,
            dest,
            dest_file: None,
        }
    }
}

impl Payload for ImagePayload {
    fn write_begin(&mut self) -> Result<(), ArchiveError> {
        // open the destination file
        // TODO: I think this will fail for a block device
        self.dest_file = Some(
            File::create(&self.dest).map_err(|err| ArchiveError::IOError {
                source: err,
                context: format!("image writer, opening path: {}", &self.dest.display()),
            })?,
        );
        debug!("opened destination: {}", self.dest.display());
        Ok(())
    }

    fn write_block(&mut self, buf: &[u8]) -> Result<Status, ArchiveError> {
        // TODO: optionally deploy to dest on worker thread

        if self.remaining < buf.len() as u64 {
            return Err(ArchiveError::PayloadDeployError {
                reason: String::from("payload write overflow"),
            });
        }

        self.dest_file
            .as_mut()
            .unwrap()
            .write_all(buf)
            .map_err(|err| {
                let pos = self.image_size - self.remaining;
                ArchiveError::IOError {
                    source: err,
                    context: format!(
                        "image writer, writing to dest: {}, pos: {}",
                        self.dest.display(),
                        pos
                    ),
                }
            })?;
        debug!("wrote {} bytes to dest", buf.len());

        self.remaining -= buf.len() as u64;
        if self.remaining == 0 {
            return Ok(Status::Complete);
        }
        Ok(Status::Pending)
    }
}

fn read_block<R: io::Read>(reader: &mut R, buf: &mut [u8]) -> Result<usize, ArchiveError> {
    let read_count = reader.read(buf).map_err(|err| ArchiveError::IOError {
        source: err,
        context: format!("image writer, reading from archive"),
    })?;
    debug!("read {} bytes from reader", read_count);
    Ok(read_count)
}

pub fn deploy_payload<'a, R: io::Read>(
    reader: &mut R,
    payload: Box<dyn Payload + 'a>,
) -> Result<(), ArchiveError> {
    let mut payload = payload;
    payload.write_begin()?;

    loop {
        let mut buf = vec![0u8; 2048];
        let read_count = read_block(reader, &mut buf)?;

        if read_count == 0 {
            return Err(ArchiveError::PayloadDeployError {
                reason: String::from("payload read completed before write finished"),
            });
        }

        let write_status = payload.write_block(&buf[0..read_count])?;
        if write_status == Status::Complete {
            return Ok(());
        }
        // else { Status::Pending, keep reading }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::test_utils::*;
    use std::process::Command;

    fn do_image_test(image_path: &PathBuf) {
        let mut img_file = File::open(image_path.clone()).unwrap();
        let file_size = img_file.metadata().unwrap().len();

        let dest_path = make_tempfile_path();
        let payload = ImagePayload::new(file_size, dest_path.clone());
        assert_eq!(
            deploy_payload(&mut img_file, Box::new(payload)).unwrap(),
            ()
        );

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

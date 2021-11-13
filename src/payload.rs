use crate::archive::ArchiveError;

// Represents the disk-image, file, directory payload data to be written to disk.
pub trait Payload {
    fn deploy(&mut self) -> Result<usize, ArchiveError>;
}

pub mod test {
    use crate::archive;
    use super::Payload;
    use std::io;

    pub struct TestPayload<R: io::Read> {
        pub reader: R,
    }
    impl<R: io::Read> TestPayload<R> {
        fn new(reader: R) -> TestPayload<R> {
            TestPayload { reader }
        }
    }
    impl<R: io::Read> Payload for TestPayload<R> {
        fn deploy(self: &mut TestPayload<R>) -> Result<usize, archive::ArchiveError> {
            let mut buf = vec![0u8; 1024000];
            let bytes_read = self.reader.read_to_end(&mut buf)?;
            Ok(bytes_read)
        }
    }
}

use crate::archive::ArchiveError;
use std::collections::HashMap;

struct Checksum(u32);

impl Checksum {
    pub fn from_str(s: &str) -> Result<Checksum, ArchiveError> {
        let cksum =
            u32::from_str_radix(s, 16).map_err(|_| ArchiveError::ChecksumFormatError {
                reason: format!("failed to parse hex checksum from: {}", s),
            })?;
        Ok(Checksum(cksum))
    }
}

pub struct Checksums {
    cksums: HashMap<String, Checksum>,
}

impl Checksums {
    pub fn parse_checksum_file(buf: &str) -> Result<Checksums, ArchiveError> {
        let mut cksums = HashMap::new();
        for line in buf.lines() {
            let mut parts = line.split_whitespace();
            let mut parse_line = |field| {
                parts
                    .next()
                    .ok_or_else(|| ArchiveError::ChecksumFormatError {
                        reason: format!("failed to parse filename {} from line: {}", field, line),
                    })
            };

            let fname = parse_line("filename")?;
            let cksum = parse_line("checksum")?;
            let cksum = Checksum::from_str(cksum)?;

            cksums.insert(String::from(fname), cksum);
        }
        Ok(Checksums { cksums })
    }

    pub fn get_checksum(&self, filename: &str) -> Option<u32> {
        self.cksums.get(filename).map(|val| val.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::*;
    use std::{fs, io::Read};

    #[test]
    fn basics() {
        init_logging();
        let mut file = fs::File::open(test_path("checksum/checksums")).unwrap();

        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        let cksums = Checksums::parse_checksum_file(&buf).unwrap();
        assert_eq!(cksums.get_checksum("manifest.json").unwrap(), 2882343476);
    }
}

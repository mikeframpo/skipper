use crate::archive::ArchiveError;
use crc32fast::Hasher;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Checksum {
    final_value: Option<u32>,
    hasher: Option<Hasher>,
}

impl Checksum {
    pub fn new_hashable() -> Checksum {
        Checksum {
            final_value: None,
            hasher: Some(Hasher::new()),
        }
    }

    pub fn update(&mut self, buf: &[u8]) {
        // update can only be called on a hashable checksum
        self.hasher.as_mut().unwrap().update(buf);
    }

    pub fn finalise(&mut self) {
        // finalise can only be called on a hashable checksum
        let hasher = self.hasher.take().unwrap();
        self.final_value = Some(hasher.finalize());
    }

    pub fn from_str(s: &str) -> Result<Checksum, ArchiveError> {
        let cksum = u32::from_str_radix(s, 16).map_err(|_| ArchiveError::ChecksumFormatError {
            reason: format!("failed to parse hex checksum from: {}", s),
        })?;
        Ok(Checksum {
            final_value: Some(cksum),
            hasher: None,
        })
    }

    pub fn to_string(&self) -> String {
        format!("{:08X?}", self.final_value.unwrap())
    }
}

impl PartialEq for Checksum {
    fn eq(&self, other: &Self) -> bool {
        self.final_value.unwrap().eq(&other.final_value.unwrap())
    }
}

pub struct ChecksumLookup {
    cksums: HashMap<String, Checksum>,
}

impl ChecksumLookup {
    pub fn parse_checksum_file(buf: &str) -> Result<ChecksumLookup, ArchiveError> {
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
        Ok(ChecksumLookup { cksums })
    }

    pub fn get_checksum(&self, filename: &str) -> Option<Checksum> {
        // return a value containing the final value but no hasher
        self.cksums.get(filename).map(|cksum| Checksum {
            final_value: cksum.final_value.clone(),
            hasher: None,
        })
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

        let cksums = ChecksumLookup::parse_checksum_file(&buf).unwrap();
        assert_eq!(
            cksums.get_checksum("manifest.json").unwrap(),
            Checksum::from_str("ABCD1234").unwrap()
        );
    }
}

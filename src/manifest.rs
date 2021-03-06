
use serde::Deserialize;
use serde_json::Result;

use crate::json;

#[derive(Deserialize)]
pub struct Manifest {
    pub payloads: Vec<PayloadInfo>,
}

#[derive(Deserialize)]
pub enum PayloadType {
    #[serde(rename = "image")]
    Image
}

#[derive(Deserialize)]
pub struct PayloadInfo {
    #[serde(rename = "type")]
    pub payload_type: PayloadType,

    pub filename: String,
    pub dest: String,

    // TODO: need to have optional fields for different types of payloads
    pub not_used: Option<String>,
}

pub fn parse_manifest(buf: &str) -> Result<Manifest> {
    json::parse_jsonc(buf)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::*;
    use std::{fs, io::Read};

    #[test]
    fn basics() {
        init_logging();
        let mut file = fs::File::open(test_path("archive/manifest.json")).unwrap();

        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        let val: Manifest = parse_manifest(&buf).unwrap();
        assert!(matches!(val.payloads[0].payload_type, PayloadType::Image));
        assert_eq!("rootfs.img", val.payloads[0].filename);
        assert_eq!("/tmp/test-device", val.payloads[0].dest);
    }
}

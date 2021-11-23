use serde::Deserialize;
use serde_json::Result;

#[derive(Deserialize)]
pub struct Manifest {
    pub payloads: Vec<PayloadInfo>,
}

#[derive(Deserialize)]
pub struct PayloadInfo {
    #[serde(rename = "type")]
    pub payload_type: String,

    pub filename: String,
    pub dest: String,

    // TODO: need to have optional fields for different types of payloads
    pub not_used: Option<String>,
}

pub fn parse_manifest(buf: &str) -> Result<Manifest> {
    serde_json::from_str(buf)
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

        let val: Manifest = serde_json::from_str(&buf).unwrap();
        assert_eq!("image", val.payloads[0].payload_type);
        assert_eq!("rootfs.img", val.payloads[0].filename);
        assert_eq!("/tmp/test-device", val.payloads[0].dest);
    }
}

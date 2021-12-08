use std::path;
use rand::{self, Rng};

pub fn init_logging() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub fn test_path<P: AsRef<path::Path>>(resource_path: P) -> path::PathBuf {
    let mut path = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test");
    path.push(resource_path);
    path
}

fn gen_rand_str(len: usize) -> String {
    let mut ret = String::new();
    let mut rng = rand::thread_rng();
    for _ in 0..len {
        let next_char: char = rng.gen_range('a'..='z');
        ret.push(next_char);
    }
    ret
}

const TMPFILE_NAMELEN: usize = 6;

pub fn make_tempfile_path() -> path::PathBuf {
    let mut tmp_path = path::PathBuf::from("/tmp");
    tmp_path.push(format!("{}.img", gen_rand_str(TMPFILE_NAMELEN)));
    tmp_path
}
use std::path;

pub fn init_logging() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub fn test_path<P: AsRef<path::Path>>(resource_path: P) -> path::PathBuf {
    let mut path = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test");
    path.push(resource_path);
    path
}
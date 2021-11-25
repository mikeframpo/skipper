use log::*;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::{fs::File, io::{self, Read}, path::Path};
use thiserror::Error;

use crate::json;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config: IOError, cause: {source}")]
    IOError {
        #[from]
        source: io::Error,
    },

    #[error("config: ConfigParseError, cause: {source}")]
    ConfigParseError { 
        #[from]
        source: serde_json::Error
    },
}

#[derive(Deserialize)]
pub struct Config {
    pub rootfs_a: String,
    pub rootfs_b: String,
}

static INSTANCE: OnceCell<Config> = OnceCell::new();

impl Config {
    pub fn get() -> &'static Config {
        INSTANCE
            .get()
            .expect("config instance was fetched before it was initialized")
    }

    pub fn load_config<P: AsRef<Path>>(config_path: Option<P>) -> Result<Config,ConfigError> {
        let config_path = match &config_path {
            Some(path) => path.as_ref(),
            None => Path::new("/data/skipper/config.jsonc"),
        };
        debug!("reading config file from {}", config_path.display());
        let mut file = File::open(config_path)?;

        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        let config: Config = json::parse_jsonc(buf.as_str())?;
        Ok(config)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn basics() {
        init_logging();

        let config_path = test_path("config/config.jsonc");
        let config = Config::load_config(Some(config_path)).unwrap();
        assert_eq!(config.rootfs_a, "/tmp/rootfs_a");
        assert_eq!(config.rootfs_b, "/tmp/rootfs_b");
    }
}

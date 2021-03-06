// TODO: remove dead code once public interfaces are established
#[allow(dead_code)]
mod cpio;

#[allow(dead_code)]
pub mod archive;

#[allow(dead_code)]
pub mod payload;

pub mod config;

pub mod json;

pub mod manifest;

#[cfg(test)]
mod test_utils;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
mod test_server;

#[allow(dead_code)]
pub mod http_reader;

#[cfg(test)]
mod linux;

#[allow(dead_code)]
pub mod checksum;

pub mod utils;
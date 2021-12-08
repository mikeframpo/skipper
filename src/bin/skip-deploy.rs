use std::{fs::File, path::PathBuf};

use clap::{App, Arg};
use skipper::{archive::Archive};

fn main() {
    let matches = App::new("Skipper deploy")
        .arg(Arg::with_name("source").required(true))
        .get_matches();

    let source = matches.value_of("source").unwrap();
    // for now only file deployments are supported
    println!("Starting deployment from file: {}", source);
    let source = File::open(PathBuf::from(source)).unwrap();

    let archive = Archive::new(source);
    while let Some(mut payload) = archive.get_next_payload().unwrap() {
        assert_eq!(payload.deploy().unwrap(), ());
    }
    println!("Deployment complete");
}

use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::{fs, io, process};

use clap::{App, Arg};
use serde_json;
use skipper::utils;
use thiserror::Error;

use skipper::archive::CHECKSUMS_FILENAME;
use skipper::checksum::Checksum;
use skipper::manifest::{parse_manifest, Manifest};

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("IO error: {message}: {source}")]
    IOError { source: io::Error, message: String },

    #[error("Argument error: {message}")]
    ArgumentError { message: String },

    #[error("Json parse error: {message}: {source}")]
    JsonParseError {
        source: serde_json::Error,
        message: String,
    },
}

fn exit_on_error(err: BuildError) -> ! {
    let message = match &err {
        BuildError::IOError { source, message } => match source.kind() {
            io::ErrorKind::NotFound => {
                format!("File not found: {}, {}", message, source.to_string())
            }
            _ => {
                format!("Error: {}, {}", message, source)
            }
        },
        BuildError::JsonParseError {
            source: _,
            message: _,
        } => err.to_string(),
        BuildError::ArgumentError { message } => format!("Argument error: {}", message),
    };
    println!("{}", message);
    process::exit(1);
}

fn map_ioerr(message: String) -> impl FnOnce(io::Error) -> BuildError {
    |err| BuildError::IOError {
        source: err,
        message,
    }
}

fn read_manifest(path: &Path) -> Result<Manifest, BuildError> {
    let mut file = fs::File::open(path).map_err(map_ioerr(path.to_string_lossy().to_string()))?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .map_err(map_ioerr(path.to_string_lossy().to_string()))?;

    let manifest = parse_manifest(&buf).map_err(|err| BuildError::JsonParseError {
        source: err,
        message: format!("failed to parse manifest"),
    })?;
    Ok(manifest)
}

fn checksum_file(file_path: &PathBuf) -> Result<Checksum, BuildError> {
    let mut file =
        File::open(file_path).map_err(map_ioerr(file_path.to_string_lossy().to_string()))?;
    let mut read_buf = [0u8; 10240];
    let mut cksum = Checksum::new_hashable();
    loop {
        let count = file
            .read(&mut read_buf)
            .map_err(map_ioerr(file_path.to_string_lossy().to_string()))?;
        if count == 0 {
            break;
        }
        cksum.update(&read_buf[0..count]);
    }
    cksum.finalise();
    Ok(cksum)
}

fn build_checksum_file(
    archive_files: &Vec<PathBuf>,
    work_dir: &PathBuf,
) -> Result<PathBuf, BuildError> {
    let cksum_file_path = work_dir.join(CHECKSUMS_FILENAME);
    let mut cksum_file =
        File::create(&cksum_file_path).map_err(map_ioerr(String::from(CHECKSUMS_FILENAME)))?;

    for filename in archive_files {
        let file_path = work_dir.join(filename);
        let cksum = checksum_file(&file_path)?;

        // note: will panic if filename is not valid unicode
        let fname = filename.file_name().unwrap().to_str().unwrap();

        write!(cksum_file, "{}\t{}\n", fname, cksum.to_string())
            .map_err(map_ioerr(String::from(CHECKSUMS_FILENAME)))?;
    }

    Ok(cksum_file_path)
}

fn setup_working_dir() -> Result<PathBuf, BuildError> {
    let mut path = PathBuf::from("/tmp");
    path.push(format!("skip-workdir-{}", utils::gen_rand_str(8)));

    fs::create_dir(&path).map_err(|err| BuildError::IOError {
        source: err,
        message: format!(
            "failed to create working directory: {}",
            path.to_string_lossy()
        ),
    })?;
    Ok(path)
}

// TODO: would be better to have workdir as a type which is cleaned up when dropped
fn cleanup_working_dir(work_dir: &Path) {
    fs::remove_dir_all(work_dir).unwrap();
}

fn generate_archive(
    archive_files: &Vec<PathBuf>,
    work_dir: &Path,
    outfile_path: &Path,
) -> Result<(), BuildError> {
    // $ echo -e "manifest.json\nimage-file" | cpio -ov --format=newc > test.cpio
    let mut proc = Command::new("cpio")
        .arg("-o")
        .arg("-v")
        .arg("--format=newc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        //.stderr(Stdio::null())
        .current_dir(work_dir)
        .spawn()
        .unwrap();

    // cpio expects the list of files to be written to the input
    let mut stdin = proc.stdin.take().expect("Failed to open cpio stdin");
    let mut input = String::new();
    for filename in archive_files {
        let fname_in = filename.to_string_lossy().to_string();
        input.push_str(&format!("{}\n", fname_in));
    }

    let input_handle = std::thread::spawn(move || {
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to cpio stdin");
    });

    input_handle.join().unwrap();
    let mut output = proc
        .wait_with_output()
        .expect("Failed to read from cpio stdout");
    let mut outfile = File::create(outfile_path).expect("Failed to create outfile");
    outfile
        .write_all(&mut output.stdout)
        .expect("Failed to write to outfile");
    Ok(())
}

fn copy_to_workdir(src: &Path, work_dir: &Path) -> PathBuf {
    let src_filename = src.file_name().unwrap();
    let dest_path = work_dir.join(src_filename);

    fs::copy(&src, &dest_path)
        .map_err(map_ioerr(format!(
            "failed to copy {} to work_dir",
            src.display()
        )))
        .unwrap_or_else(|err| exit_on_error(err));

    dest_path
}

fn build_archive(root_path: &Path, output: &Path) {
    // TODO: should tidy this function up so it returns an error, and just exit at top level
    if !root_path.is_dir() {
        exit_on_error(BuildError::ArgumentError {
            message: format!(
                "file-root provided was not a valid directory: {}",
                root_path.to_string_lossy()
            ),
        });
    }

    let work_dir = setup_working_dir().unwrap_or_else(|err| exit_on_error(err));

    let manifest_path = root_path.join("manifest.jsonc");
    let manifest_path = copy_to_workdir(&manifest_path, &work_dir);

    let manifest = read_manifest(&manifest_path).unwrap_or_else(|err| exit_on_error(err));

    let mut archive_files = vec![PathBuf::from(manifest_path.file_name().unwrap())];
    // generate list of files to go in the archive
    for payload_info in manifest.payloads {
        match payload_info.payload_type {
            skipper::manifest::PayloadType::Image => {
                // copy to work dir
                let src_path = root_path.join(payload_info.filename);
                let dest_path = copy_to_workdir(&src_path, &work_dir);

                // push the filename
                archive_files.push(PathBuf::from(dest_path.file_name().unwrap()));
            }
        }
    }

    // checksum the files
    let checksums_path =
        build_checksum_file(&archive_files, &work_dir).unwrap_or_else(|err| exit_on_error(err));
    archive_files.insert(0, PathBuf::from(checksums_path.file_name().unwrap()));

    // generate the archive
    generate_archive(&archive_files, &work_dir, output).unwrap_or_else(|err| exit_on_error(err));

    cleanup_working_dir(&work_dir);
}

fn main() {
    let matches = App::new("skip-build")
        .arg(
            Arg::with_name("file-root")
                .required(true)
                .takes_value(true)
                .short("-p")
                .help("path to root directory containing files to be packaged in archive"),
        )
        .arg(
            Arg::with_name("output")
                .required(true)
                .takes_value(true)
                .short("-o")
                .help("output archive file"),
        )
        .get_matches();

    let get_filename_path = |arg| {
        let value = matches.value_of(arg).unwrap();
        Path::new(value)
    };
    let file_root_path = get_filename_path("file-root");
    let output = get_filename_path("output");

    build_archive(file_root_path, output);
}

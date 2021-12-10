use std::{
    collections::HashSet,
    io::BufRead,
    path::{self},
    process::{Child, Command, Stdio},
    sync::Mutex,
};

use log::debug;

use crate::{
    linux::{self},
    test_utils::test_path,
};

lazy_static! {
    static ref SERVER_PORTS: Mutex<HashSet<u32>> = Mutex::new(HashSet::new());
}

fn get_server_port() -> u32 {
    // loop from 8000 until we get a free port
    // TODO: this should really look at available ports on the system
    let mut ports = SERVER_PORTS.lock().unwrap();
    for port in 8000u32..=8080 {
        if !ports.contains(&port) {
            ports.insert(port);
            return port;
        }
    }
    panic!("all test server ports are in use")
}

fn free_server_port(port: u32) {
    let mut ports = SERVER_PORTS.lock().unwrap();
    if !ports.contains(&port) {
        panic!("server port was not found in map");
    }
    ports.remove(&port);
}

pub struct TestServer {
    process: Child,
    pub port: u32,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // send the INT signal, which is how the process is usually stopped on a terminal
        linux::signal(&self.process, linux::Signal::INT);
        // TODO: this should wait for some time and then send a SIGKILL if it hasn't terminated yet
        // currently it will just block
        let ret_code = self
            .process
            .wait()
            .expect("failed to wait for server process to exit");

        free_server_port(self.port);

        debug!("server exited with ret code: {}", ret_code);
        // we don't assert a zero exit code, because rust returns a None exit code if the process
        // was killed by a signal
    }
}

pub fn create_test_server<P: AsRef<path::Path>>(server_root: P) -> TestServer {
    let server_port = get_server_port();
    let server_module = test_path("http-server");
    let server_root = test_path(server_root);

    debug!(
        "starting test server on port: {}, root path: {}",
        server_port,
        server_root.to_str().unwrap()
    );

    let mut server = Command::new("python3")
        .env("PYTHONPATH", server_module)
        .arg("-u")
        .arg("-m")
        .arg("RangeHTTPServer")
        .arg(server_port.to_string())
        .current_dir(server_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // the server logs requests to stderr, which becomes noise in the test output
        .spawn()
        .expect("failed to start server process");

    let stdout = server.stdout.as_mut().unwrap();
    let mut reader = std::io::BufReader::new(stdout);

    // note the -u flag to the python interpreter is required, else the piped
    // output is buffered and we never see it until the process exits
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.starts_with("Serving HTTP"));

    TestServer {
        process: server,
        port: server_port,
    }
}

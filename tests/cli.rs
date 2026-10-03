//! The executable itself, run the way a person runs it.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const VERSION: &str = concat!("fugantt ", env!("CARGO_PKG_VERSION"));

/// An empty directory to stand in, so no settings file or database of the
/// developer's is picked up.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fugantt(dir: &PathBuf) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fugantt"));
    command
        .current_dir(dir)
        .env("FUGANTT_DB", dir.join("fugantt.db"))
        .env("FUGANTT_OPEN", "0")
        .env_remove("FUGANTT_NO_AUTH")
        .env_remove("HOST");
    command
}

/// A running server, stopped when the test is done with it.
struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    fn start(name: &str, env: &[(&str, &str)]) -> Self {
        let dir = scratch(name);
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let child = fugantt(&dir)
            .env("PORT", port.to_string())
            .envs(env.iter().copied())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        Self { child, port }
    }

    fn get(&self, path: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut stream = loop {
            match TcpStream::connect(("127.0.0.1", self.port)) {
                Ok(stream) => break stream,
                Err(error) if Instant::now() > deadline => panic!("起動しなかった: {error}"),
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        };

        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            self.port
        )
        .unwrap();

        let mut body = String::new();
        stream.read_to_string(&mut body).unwrap();
        body
    }
}

#[test]
fn version_is_a_command() {
    let dir = scratch("version");

    let word = fugantt(&dir).arg("version").output().unwrap();
    let flag = fugantt(&dir).arg("--version").output().unwrap();

    assert_eq!(
        String::from_utf8_lossy(&word.stdout),
        format!("{VERSION}\n")
    );
    assert_eq!(word.stdout, flag.stdout);
}

/// Before anything else, so it is the line at the top of the log too.
#[test]
fn a_start_says_its_version_first() {
    let mut server = Server::start("start", &[]);

    let mut first = String::new();
    BufReader::new(server.child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();

    assert_eq!(first.trim_end(), VERSION);
}

#[test]
fn the_drawer_shows_the_version() {
    let server = Server::start(
        "drawer",
        &[("FUGANTT_NO_AUTH", "yes-everyone-on-this-network-can-edit")],
    );

    let page = server.get("/");

    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(page.contains(VERSION), "ドロワーに版が出ていない");
}

/// Somebody who has not signed in is not told which version to look up.
#[test]
fn the_login_page_does_not_show_the_version() {
    let server = Server::start("login", &[]);

    let page = server.get("/login");

    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(!page.contains(VERSION));
}

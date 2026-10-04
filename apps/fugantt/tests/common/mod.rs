//! Running the executable the way a person runs it, for the tests that do.

#![allow(dead_code)]

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// An empty directory to stand in, so no settings file or database of the
/// developer's is picked up.
pub fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn fugantt(dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fugantt"));
    command
        .current_dir(dir)
        .env("FUGANTT_DB", dir.join("fugantt.db"))
        .env("FUGANTT_OPEN", "0")
        .env_remove("FUGANTT_NO_AUTH")
        .env_remove("FUGANTT_TODAY")
        .env_remove("FUGANTT_CONF")
        .env_remove("HOST");
    command
}

/// A running server, stopped when the test is done with it.
pub struct Server {
    pub child: Child,
    pub port: u16,
    pub dir: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    pub fn start(name: &str, env: &[(&str, &str)]) -> Self {
        Self::start_with(name, env, |_| {})
    }

    /// Starts in a fresh directory, after `prepare` has put what it wants there.
    pub fn start_with(name: &str, env: &[(&str, &str)], prepare: impl FnOnce(&Path)) -> Self {
        let dir = scratch(name);
        prepare(&dir);

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

        Self { child, port, dir }
    }

    fn connect(&self) -> TcpStream {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            match TcpStream::connect(("127.0.0.1", self.port)) {
                Ok(stream) => break stream,
                Err(error) if Instant::now() > deadline => panic!("起動しなかった: {error}"),
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    }

    /// The raw response, status line and headers included.
    pub fn get(&self, path: &str) -> String {
        String::from_utf8_lossy(&self.raw(path)).into_owned()
    }

    /// The status and the body, with the transfer encoding taken off.
    pub fn body(&self, path: &str) -> (u16, String) {
        let raw = self.raw(path);
        let split = raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("ヘッダーの終わりが無い");
        let head = String::from_utf8_lossy(&raw[..split]).to_lowercase();
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("ステータスが読めない");

        let rest = &raw[split + 4..];
        let bytes = if head.contains("transfer-encoding: chunked") {
            unchunk(rest)
        } else {
            rest.to_vec()
        };

        (
            status,
            String::from_utf8(bytes).expect("本文が UTF-8 でない"),
        )
    }

    fn raw(&self, path: &str) -> Vec<u8> {
        let mut stream = self.connect();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: */*\r\nConnection: close\r\n\r\n",
            self.port
        )
        .unwrap();

        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        raw
    }
}

/// Joins a chunked body back together.
fn unchunk(mut rest: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    while let Some(line_end) = rest.windows(2).position(|window| window == b"\r\n") {
        let size = std::str::from_utf8(&rest[..line_end])
            .ok()
            .and_then(|line| usize::from_str_radix(line.split(';').next()?.trim(), 16).ok())
            .unwrap_or(0);
        if size == 0 {
            break;
        }

        let start = line_end + 2;
        out.extend_from_slice(&rest[start..start + size]);
        rest = &rest[start + size + 2..];
    }

    out
}

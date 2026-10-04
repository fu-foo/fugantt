//! The example, asked the things the grid asks.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Host {
    child: Child,
    port: u16,
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Host {
    fn start() -> Self {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let child = Command::new(env!("CARGO_BIN_EXE_minimal-host"))
            .env("PORT", port.to_string())
            .env("HOST", "127.0.0.1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        Self { child, port }
    }

    /// The status and the body of one request.
    fn ask(&self, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut stream = loop {
            match TcpStream::connect(("127.0.0.1", self.port)) {
                Ok(stream) => break stream,
                Err(error) if Instant::now() > deadline => panic!("起動しなかった: {error}"),
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        };

        let body = body.unwrap_or("");
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.port,
            body.len()
        )
        .unwrap();

        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();

        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&raw[..split]).to_lowercase();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let rest = &raw[split + 4..];
        let bytes = if head.contains("transfer-encoding: chunked") {
            unchunk(rest)
        } else {
            rest.to_vec()
        };

        (status, String::from_utf8(bytes).unwrap())
    }

    fn json(&self, method: &str, path: &str, body: Option<&str>) -> serde_json::Value {
        let (status, text) = self.ask(method, path, body);
        assert_eq!(status, 200, "{method} {path}: {text}");
        serde_json::from_str(&text).unwrap()
    }
}

fn unchunk(mut rest: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    while let Some(line_end) = rest.windows(2).position(|w| w == b"\r\n") {
        let size = std::str::from_utf8(&rest[..line_end])
            .ok()
            .and_then(|line| usize::from_str_radix(line.trim(), 16).ok())
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

const API: &str = "/g/api/plans/demo";

fn names(grid: &serde_json::Value) -> Vec<String> {
    grid["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|task| task["name"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn the_page_points_the_grid_at_this_host_s_own_address() {
    let host = Host::start();
    let (status, page) = host.ask("GET", "/", None);

    assert_eq!(status, 200);
    assert!(page.contains(r#"id="fugantt-grid""#), "{page}");
    assert!(page.contains(&format!(r#"data-api="{API}""#)), "{page}");

    let (status, script) = host.ask("GET", "/g/grid.js", None);
    assert_eq!(status, 200);
    assert!(script.contains("fugantt-grid"));
}

/// Nothing answers at the address fugantt itself uses: the grid is here
/// because it was told where to look.
#[test]
fn fugantt_s_own_address_is_not_here() {
    let host = Host::start();

    assert_eq!(host.ask("GET", "/api/projects/demo/grid", None).0, 404);
}

#[test]
fn the_plan_is_read_added_to_edited_and_reordered() {
    let host = Host::start();

    let grid = host.json("GET", &format!("{API}/grid"), None);
    assert_eq!(names(&grid), ["設計", "実装", "テスト"]);
    let first = grid["tasks"][0]["id"].as_str().unwrap().to_owned();

    // A row added after the first lands second.
    let added = host.json(
        "POST",
        &format!("{API}/tasks"),
        Some(&format!(r#"{{"after":"{first}"}}"#)),
    );
    let new = added["task_id"].as_str().unwrap().to_owned();
    assert_eq!(names(&added["grid"]).len(), 4);
    assert_eq!(added["grid"]["tasks"][1]["id"], new);

    // Named, and given dates the way a person types them.
    host.json(
        "POST",
        &format!("{API}/tasks/{new}"),
        Some(r#"{"field":"name","value":"レビュー"}"#),
    );
    let edited = host.json(
        "POST",
        &format!("{API}/tasks/{new}"),
        Some(r#"{"field":"schedule","value":"2026-10-05/20261009"}"#),
    );
    let row = &edited["grid"]["tasks"][1];
    assert_eq!(row["name"], "レビュー");
    assert_eq!(row["start"], "2026-10-05");
    assert_eq!(row["end"], "2026-10-09");

    // Moved up, it is first.
    let moved = host.json(
        "POST",
        &format!("{API}/tasks/{new}/move"),
        Some(r#"{"action":"up"}"#),
    );
    assert_eq!(names(&moved["grid"])[0], "レビュー");

    // The revision moved with every write, so a watcher would know.
    assert!(moved["grid"]["revision"].as_i64() > grid["revision"].as_i64());
}

#[test]
fn text_that_is_not_a_date_is_refused_and_changes_nothing() {
    let host = Host::start();
    let grid = host.json("GET", &format!("{API}/grid"), None);
    let first = grid["tasks"][0]["id"].as_str().unwrap();

    let (status, _) = host.ask(
        "POST",
        &format!("{API}/tasks/{first}"),
        Some(r#"{"field":"start","value":"あした"}"#),
    );
    assert_eq!(status, 400);

    let after = host.json("GET", &format!("{API}/grid"), None);
    assert_eq!(after["tasks"][0]["start"], grid["tasks"][0]["start"]);
    assert_eq!(after["revision"], grid["revision"]);
}

/// The grid draws read-only unless it is told otherwise, and a host that
/// forgets to say so gets a chart nobody can type into.
#[test]
fn the_plan_says_it_may_be_edited() {
    let host = Host::start();

    let grid = host.json("GET", &format!("{API}/grid"), None);
    assert_eq!(grid["can_edit"], true);
}

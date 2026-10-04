//! What the server says about a fixed database, held against what it said
//! when the answers were written down.
//!
//! The code underneath is being moved between crates. None of that is meant
//! to change a byte of what a person or a program is handed, and this is the
//! test that would notice.
//!
//! To write the answers down afresh: `UPDATE_GOLDEN=1 cargo test --test golden`.
//! That is for a deliberate change to the output, never for a failure.

mod common;

use std::path::{Path, PathBuf};

use common::Server;

const TODAY: &str = "2026-09-15";
const NO_AUTH: &str = "yes-everyone-on-this-network-can-edit";

fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// A server reading a copy of the fixture. The committed file is never opened:
/// starting up migrates and fills in holidays, and both write.
fn server(name: &str) -> Server {
    Server::start_with(
        name,
        &[("FUGANTT_NO_AUTH", NO_AUTH), ("FUGANTT_TODAY", TODAY)],
        |dir: &Path| {
            std::fs::copy(here().join("fixtures/v1.db"), dir.join("fugantt.db")).unwrap();
        },
    )
}

/// JSON, laid out one value to a line so a difference reads as a difference.
fn json(server: &Server, path: &str) -> String {
    let (status, body) = server.body(path);
    assert_eq!(status, 200, "{path}: {body}");

    let value: serde_json::Value = serde_json::from_str(&body).expect("JSON でない");
    serde_json::to_string_pretty(&value).unwrap() + "\n"
}

/// A page, with the two things that move for reasons of their own taken out:
/// the hash in each static URL, and the version.
fn html(server: &Server, path: &str) -> String {
    let (status, body) = server.body(path);
    assert_eq!(status, 200, "{path}: {body}");

    let version = concat!("fugantt ", env!("CARGO_PKG_VERSION"));
    unhash(&body.replace(version, "fugantt VERSION")) + "\n"
}

/// `/static/0123456789abcdef/name` → `/static/HASH/name`.
fn unhash(page: &str) -> String {
    const MARK: &str = "/static/";
    let mut out = String::with_capacity(page.len());
    let mut rest = page;

    while let Some(at) = rest.find(MARK) {
        let after = &rest[at + MARK.len()..];
        let hash = after.bytes().take_while(u8::is_ascii_hexdigit).count();

        out.push_str(&rest[..at + MARK.len()]);
        if hash == 16 && after[hash..].starts_with('/') {
            out.push_str("HASH");
            rest = &after[hash..];
        } else {
            rest = after;
        }
    }

    out.push_str(rest);
    out
}

fn check(name: &str, actual: &str) {
    let path = here().join("golden").join(name);

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }

    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{name} の記録が無い。UPDATE_GOLDEN=1 で作る"));

    if expected != actual {
        let was = path.with_extension("actual");
        std::fs::write(&was, actual).unwrap();
        panic!(
            "{name} が記録と違う。\n  diff {} {}",
            path.display(),
            was.display()
        );
    }
}

#[test]
fn the_grid_says_what_it_said() {
    let server = server("golden-grid");

    check("grid.json", &json(&server, "/api/projects/golden/grid"));
    check(
        "grid-empty.json",
        &json(&server, "/api/projects/empty/grid"),
    );
}

#[test]
fn the_document_says_what_it_said() {
    let server = server("golden-document");

    check(
        "document.json",
        &json(&server, "/api/projects/golden/document"),
    );
    check(
        "document-tasks.json",
        &json(&server, "/api/projects/golden/document?settings=0"),
    );
}

#[test]
fn the_numbers_say_what_they_said() {
    let server = server("golden-summary");

    check("projects.json", &json(&server, "/api/projects"));
    check("summary.json", &json(&server, "/api/summary"));
}

#[test]
fn the_pages_say_what_they_said() {
    let server = server("golden-pages");

    for (name, path) in [
        ("page-home.html", "/"),
        ("page-project.html", "/projects/golden"),
        ("page-stats.html", "/projects/golden/stats"),
        ("page-capacity.html", "/projects/golden/capacity"),
        ("page-settings.html", "/projects/golden/settings"),
        ("page-history.html", "/projects/golden/history"),
        ("page-admin.html", "/admin"),
    ] {
        check(name, &html(&server, path));
    }
}

/// The day the tests pin is the day the server works from.
#[test]
fn today_is_the_pinned_day() {
    let server = server("golden-today");

    let grid = json(&server, "/api/projects/golden/grid");
    assert!(grid.contains(&format!("\"today\": \"{TODAY}\"")), "{grid}");
}

/// Reading the fixture must leave the committed file exactly as it was.
#[test]
fn the_fixture_is_never_written_to() {
    let fixture = here().join("fixtures/v1.db");
    let before = std::fs::read(&fixture).unwrap();

    let server = server("golden-untouched");
    let _ = server.body("/api/projects/golden/grid");
    drop(server);

    assert!(
        std::fs::read(&fixture).unwrap() == before,
        "固定 DB が書き換わった"
    );
    assert!(
        !fixture.with_extension("db-wal").exists(),
        "WAL が出来ている"
    );
}

#[test]
fn a_static_hash_is_taken_out_and_nothing_else() {
    assert_eq!(
        unhash(r#"<link href="/static/0123456789abcdef/grid.css">"#),
        r#"<link href="/static/HASH/grid.css">"#
    );
    // Not sixteen hex digits: left alone.
    assert_eq!(unhash("/static/abc/grid.css"), "/static/abc/grid.css");
    assert_eq!(unhash("no urls here"), "no urls here");
}

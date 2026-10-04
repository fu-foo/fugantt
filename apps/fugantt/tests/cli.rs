//! The executable itself, run the way a person runs it.

mod common;

use std::io::{BufRead, BufReader};

use common::{Server, fugantt, scratch};

const VERSION: &str = concat!("fugantt ", env!("CARGO_PKG_VERSION"));

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

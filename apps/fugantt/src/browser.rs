//! Opening the schedule for whoever just started the server.
//!
//! The program is one executable and its interface is a web page, which leaves
//! a gap nobody should have to cross by hand: a person double-clicks it, gets a
//! console window, and is expected to know that the point of it is a URL typed
//! somewhere else. So it opens the page itself, once the port answers.
//!
//! Only when the server is bound to loopback. A container or a LAN host is
//! nobody's desktop, and there is no browser there to open.

use std::{process::Command, time::Duration};

/// What to do about the page when the server starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Open {
    /// A window of its own, with no address bar. The nearest thing to an
    /// application the browser can give without one being installed.
    Window,
    /// An ordinary tab.
    Tab,
    Nothing,
}

/// Reads `FUGANTT_OPEN`, and decides from the address when it says nothing.
///
/// `window`, `tab`, `0` — the last for anyone who does not want a browser
/// jumping in front of what they were doing.
pub fn wanted(host: &str) -> Open {
    match std::env::var("FUGANTT_OPEN").as_deref().map(str::trim) {
        Ok("window") => Open::Window,
        Ok("tab" | "browser") => Open::Tab,
        Ok("0" | "no" | "false" | "") => Open::Nothing,
        Ok(_) | Err(_) => {
            // The development server restarts on every save. A browser window
            // per keystroke-and-a-half is not help.
            if std::env::var_os("TOPCOAT_DEV_URL").is_some() {
                return Open::Nothing;
            }

            if is_loopback(host) {
                // A separate window on Windows, where the one browser that is
                // certainly installed can give one. Elsewhere a tab: Safari has
                // no such mode, and guessing at which browser is there to ask
                // is worse than opening the one the person chose.
                if cfg!(windows) {
                    Open::Window
                } else {
                    Open::Tab
                }
            } else {
                Open::Nothing
            }
        }
    }
}

/// Whether this address is this machine talking to itself.
fn is_loopback(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "::1" | "[::1]" | "localhost")
}

/// Waits for the port to answer, then opens the page.
///
/// Waiting rather than sleeping a fixed moment: a browser opened before the
/// listener is up lands on a connection error, and the person then has to know
/// to reload — which is exactly the knowledge this is here to spare them.
pub async fn open_when_ready(open: Open, host: &str, port: u16) {
    if open == Open::Nothing {
        return;
    }

    let address = format!("{host}:{port}");
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(&address).await.is_ok() {
            launch(open, &url(host, port));
            return;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// The address to show a person, which is not always the one that was bound.
pub fn url(host: &str, port: u16) -> String {
    let shown = match host {
        // Nothing is served on 0.0.0.0; it means "every address this machine
        // has", and printing it sends people to a page that will not load.
        "0.0.0.0" | "::" => "127.0.0.1",
        "[::1]" => "::1",
        other => other,
    };

    if shown.contains(':') {
        format!("http://[{shown}]:{port}")
    } else {
        format!("http://{shown}:{port}")
    }
}

fn launch(open: Open, url: &str) {
    if open == Open::Window && cfg!(windows) && windowed(url) {
        return;
    }

    // Failure is silent on purpose: the address has already been printed, and a
    // stack of errors about a browser is not what someone wants from a program
    // that has otherwise started correctly.
    if cfg!(windows) {
        // Through the shell, which knows the file associations. The empty
        // string is the window title `start` would otherwise take the URL for.
        let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    } else if cfg!(target_os = "macos") {
        let _ = Command::new("open").arg(url).spawn();
    } else {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

/// Edge in application mode: a window with no address bar and its own place on
/// the taskbar. It is on every supported Windows, so this is not a gamble —
/// but it is still checked, and an ordinary tab is a fine second best.
fn windowed(url: &str) -> bool {
    Command::new("msedge")
        .arg(format!("--app={url}"))
        .spawn()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bound address decides, unless a person said otherwise.
    #[test]
    fn a_server_on_the_network_opens_nothing() {
        // `cargo test` runs outside the watcher, but a developer's shell may
        // still carry its variable.
        if std::env::var_os("TOPCOAT_DEV_URL").is_some() {
            return;
        }

        assert_eq!(wanted("0.0.0.0"), Open::Nothing);
        assert_eq!(wanted("192.168.1.10"), Open::Nothing);
        assert_ne!(wanted("127.0.0.1"), Open::Nothing);
        assert_ne!(wanted("localhost"), Open::Nothing);
    }

    /// What is printed has to be somewhere a browser can go.
    #[test]
    fn the_address_shown_is_one_that_answers() {
        assert_eq!(url("0.0.0.0", 3000), "http://127.0.0.1:3000");
        assert_eq!(url("127.0.0.1", 8080), "http://127.0.0.1:8080");
        assert_eq!(url("::1", 3000), "http://[::1]:3000");
        assert_eq!(url("::", 3000), "http://127.0.0.1:3000");
    }
}

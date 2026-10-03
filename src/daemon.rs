//! `fugantt start`, `stop`, `status`, `log`: running in the background.
//!
//! Plain `fugantt` stays what it was — in the foreground, stopped by closing it
//! or Ctrl+C — because Docker, Fly and anyone who double-clicks the executable
//! rely on exactly that. These are for the person who would rather not keep a
//! console open for a server they look at in a browser.
//!
//! The background server is this same executable started again, cut loose from
//! the terminal, with its output going to a file. Which process it is and where
//! its output went are written next to the data, named by port, so two servers
//! on two ports do not trip over each other.
//!
//! Nothing here needs a library: whether a process is alive, what it is called
//! and how to stop it are asked of the commands every system already has.

use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// How long `start` waits for the port to answer before calling it a failure.
const READY: Duration = Duration::from_secs(20);

/// How long `stop` waits for a polite exit before insisting.
const POLITE: Duration = Duration::from_secs(5);

struct Paths {
    pid: PathBuf,
    log: PathBuf,
}

fn host() -> String {
    std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned())
}

fn port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(1861)
}

fn paths() -> Paths {
    let dir = crate::db::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let _ = fs::create_dir_all(&dir);
    let port = port();

    Paths {
        pid: dir.join(format!("fugantt-{port}.pid")),
        log: dir.join(format!("fugantt-{port}.log")),
    }
}

/// The background server for this port, if there is one and it is ours.
///
/// A recorded number on its own proves nothing: after a restart the machine
/// hands numbers out again, and the one in the file may now be somebody's
/// editor. It counts only while the process under it is called fugantt.
fn running(paths: &Paths) -> Option<u32> {
    let pid: u32 = fs::read_to_string(&paths.pid).ok()?.trim().parse().ok()?;

    if is_fugantt(pid) {
        Some(pid)
    } else {
        // Left over from a server that is gone. Tidied away, so the next
        // `start` does not have to think about it.
        let _ = fs::remove_file(&paths.pid);
        None
    }
}

#[cfg(unix)]
fn is_fugantt(pid: u32) -> bool {
    Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .is_some_and(|out| String::from_utf8_lossy(&out.stdout).contains("fugantt"))
}

#[cfg(windows)]
fn is_fugantt(pid: u32) -> bool {
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .ok()
        .is_some_and(|out| {
            String::from_utf8_lossy(&out.stdout)
                .to_ascii_lowercase()
                .contains("fugantt")
        })
}

/// Whether something is listening on the port.
fn answers(port: u16) -> bool {
    use std::net::ToSocketAddrs;

    let host = match host().as_str() {
        "0.0.0.0" => "127.0.0.1".to_owned(),
        "::" => "::1".to_owned(),
        other => other.to_owned(),
    };
    let Some(address) = (host.as_str(), port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut found| found.next())
    else {
        return false;
    };

    std::net::TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok()
}

/// For the foreground server that found its port taken: whether the one
/// holding it is our own, started with `fugantt start`.
pub fn background_pid() -> Option<u32> {
    running(&paths())
}

/// `fugantt start`.
pub fn start() -> i32 {
    let paths = paths();
    let url = crate::browser::url(&host(), port());

    if let Some(pid) = running(&paths) {
        println!("もう動いています（{url}、プロセス {pid}）。");
        println!("止めるには: fugantt stop");
        return 0;
    }

    if answers(port()) {
        eprintln!("{} は他のプログラムが使っています。", port());
        eprintln!("別の番号にするには、fugantt.ini に PORT = 1862 のように書いてください。");
        return 1;
    }

    let Ok(exe) = std::env::current_exe() else {
        eprintln!("自分の実行ファイルの場所が分かりませんでした。");
        return 1;
    };

    let log = match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.log)
    {
        Ok(file) => file,
        Err(error) => {
            eprintln!("ログを書けませんでした（{}）: {error}", paths.log.display());
            return 1;
        }
    };
    let Ok(err) = log.try_clone() else {
        eprintln!("ログを書けませんでした（{}）。", paths.log.display());
        return 1;
    };
    // Where the new output begins, so a failure shows this attempt's words
    // and not last week's.
    let from = log.metadata().map(|meta| meta.len()).unwrap_or(0);

    let mut command = Command::new(exe);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err));
    detach(&mut command);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!("起動できませんでした: {error}");
            return 1;
        }
    };

    let pid = child.id();
    if let Err(error) = fs::write(&paths.pid, pid.to_string()) {
        eprintln!(
            "プロセス番号を書けませんでした（{}）: {error}",
            paths.pid.display()
        );
    }

    let began = Instant::now();
    while began.elapsed() < READY {
        if let Ok(Some(_)) = child.try_wait() {
            let _ = fs::remove_file(&paths.pid);
            eprintln!("起動に失敗しました。ログの最後:");
            eprint!("{}", tail_from(&paths.log, from, 20));
            return 1;
        }

        if answers(port()) {
            println!("起動しました（プロセス {pid}）。");
            println!("画面: {url}");
            if let Some(data) = first_line_starting(&paths.log, from, "データ: ") {
                println!("{data}");
            }
            println!("ログ: {}", paths.log.display());
            println!("止めるには: fugantt stop");
            return 0;
        }

        std::thread::sleep(Duration::from_millis(150));
    }

    eprintln!(
        "{} 秒待っても応答がありません。ログ: {}",
        READY.as_secs(),
        paths.log.display()
    );
    1
}

/// Cut loose from the terminal it was started from: closing that window, or
/// the Ctrl+C meant for the next command, must not reach the server.
#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    // No console window of its own, and not in the group a Ctrl+C reaches.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
}

/// `fugantt stop`.
pub fn stop() -> i32 {
    let paths = paths();

    let Some(pid) = running(&paths) else {
        println!("動いていません。");
        return 0;
    };

    ask_to_stop(pid, false);

    let began = Instant::now();
    while is_fugantt(pid) {
        if began.elapsed() > POLITE {
            ask_to_stop(pid, true);
            std::thread::sleep(Duration::from_millis(300));
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    if is_fugantt(pid) {
        eprintln!("止められませんでした（プロセス {pid}）。");
        return 1;
    }

    let _ = fs::remove_file(&paths.pid);
    println!("止めました。");
    0
}

#[cfg(unix)]
fn ask_to_stop(pid: u32, insist: bool) {
    let signal = if insist { "-KILL" } else { "-TERM" };
    let _ = Command::new("kill")
        .args([signal, &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(windows)]
fn ask_to_stop(pid: u32, _insist: bool) {
    // A process with no window has nothing to receive a polite request, so
    // Windows goes straight to the firm one. SQLite's journal makes that safe.
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// `fugantt status`.
pub fn status() -> i32 {
    let paths = paths();
    let url = crate::browser::url(&host(), port());

    match running(&paths) {
        Some(pid) => {
            println!("動いています（プロセス {pid}）。");
            // The one that is running, which is not always the one on disk: a
            // replaced executable changes nothing until the next start.
            if let Some(version) = version_in(&tail_from(&paths.log, 0, 500)) {
                println!("{version}");
            }
            println!("画面: {url}");
            if let Some(data) = last_line_starting(&paths.log, "データ: ") {
                println!("{data}");
            }
            println!("ログ: {}", paths.log.display());
            0
        }
        None => {
            println!("動いていません。");
            // Something may still be on the port: a foreground fugantt, or
            // another program. Worth a line, since `start` would refuse.
            if answers(port()) {
                println!(
                    "（{} には別のものが応答しています。前面で動いている fugantt かもしれません）",
                    port()
                );
            }
            // Not running is an answer, not a failure.
            3
        }
    }
}

/// `fugantt log`: the end of the background server's output.
pub fn log() -> i32 {
    let paths = paths();

    if !paths.log.exists() {
        println!("ログはまだありません（{}）。", paths.log.display());
        return 0;
    }

    print!("{}", tail_from(&paths.log, 0, 50));
    0
}

/// The last `lines` lines of a file, read from byte `from` on.
fn tail_from(path: &PathBuf, from: u64, lines: usize) -> String {
    let Ok(mut file) = fs::File::open(path) else {
        return String::new();
    };
    // Only the end is wanted; a log that has run for months is not read whole.
    let length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let start = from.max(length.saturating_sub(64 * 1024));
    let _ = file.seek(SeekFrom::Start(start));

    let mut text = String::new();
    let _ = file.read_to_string(&mut text);

    let all: Vec<&str> = text.lines().collect();
    let shown = &all[all.len().saturating_sub(lines)..];
    shown.iter().map(|line| format!("{line}\n")).collect()
}

fn first_line_starting(path: &PathBuf, from: u64, prefix: &str) -> Option<String> {
    tail_from(path, from, 200)
        .lines()
        .find(|line| line.starts_with(prefix))
        .map(ToOwned::to_owned)
}

/// The version the running server announced when it started.
fn version_in(log: &str) -> Option<&str> {
    log.lines().rev().find(|line| {
        line.strip_prefix("fugantt ")
            .is_some_and(|rest| rest.starts_with(|first: char| first.is_ascii_digit()))
    })
}

fn last_line_starting(path: &PathBuf, prefix: &str) -> Option<String> {
    tail_from(path, 0, 500)
        .lines()
        .rev()
        .find(|line| line.starts_with(prefix))
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The newest start is the one that is running.
    #[test]
    fn the_running_version_is_the_last_one_the_log_names() {
        let log = concat!(
            "fugantt 0.9.0\n",
            "データ: /srv/fugantt.db\n",
            "fugantt 1.0.0\n",
            "データ: /srv/fugantt.db\n",
            "画面: http://127.0.0.1:1861\n",
        );

        assert_eq!(version_in(log), Some("fugantt 1.0.0"));
    }

    /// Other lines open with the name too; only a version follows it with a digit.
    #[test]
    fn a_line_that_only_starts_with_the_name_is_not_a_version() {
        let log = "fugantt 1.0.0\nfugantt が裏で動いています（プロセス 12）。\n";

        assert_eq!(version_in(log), Some("fugantt 1.0.0"));
        assert_eq!(version_in("データ: /srv/fugantt.db\n"), None);
    }
}

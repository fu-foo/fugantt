//! Settings from a file, for people who do not have a shell.
//!
//! Everything here is an environment variable, which is the right shape for a
//! container and the wrong shape for the person this is mostly for: one
//! executable on a Windows machine, run by double-clicking it. Setting `PORT`
//! there means either a batch file or a trip through System Properties, and
//! neither is something to ask of somebody who was told this needs nothing
//! installed.
//!
//! So the same variables can be written in a file next to it. Not a new
//! settings system — the keys are the variable names, exactly, because two
//! vocabularies for one setting is how a README starts lying.

use std::path::PathBuf;

/// The names this file may go by.
///
/// `.ini` is the one in the documentation: on Windows it says "text you may
/// edit" to everyone, and on Unix it is at worst old-fashioned. The others are
/// read because somebody will reach for them out of habit.
const NAMES: [&str; 3] = ["fugantt.ini", "fugantt.conf", ".env"];

/// The port this listens on when nobody says otherwise.
///
/// 1861 is the year Henry Gantt was born, which makes it easy to remember and,
/// more to the point, empty. 3000 and 8080 are where every second development
/// tool lives: two of them on one machine means the wrong program answers, and
/// the failure looks like the app being broken rather than the port being
/// taken — an afternoon was lost to exactly that on the machine this was
/// written on.
const DEFAULT_PORT: &str = "1861";

/// The settings a file may carry.
///
/// A closed list, so a typo is caught and said out loud rather than silently
/// doing nothing — which is the single most common way a config file wastes
/// somebody's afternoon.
const KEYS: [&str; 6] = [
    "HOST",
    "PORT",
    "FUGANTT_DB",
    "FUGANTT_OPEN",
    "FUGANTT_ALLOW_HTTP",
    "FUGANTT_NO_AUTH",
];

/// What a run of `load` did, for the startup line and for `--config`.
pub struct Loaded {
    pub path: Option<PathBuf>,
    /// Files that were found but not read, because one was chosen first.
    pub ignored: Vec<PathBuf>,
    /// Keys the file set, and keys it could not.
    pub applied: Vec<String>,
    pub overridden: Vec<String>,
    pub unknown: Vec<String>,
}

/// Reads the first settings file there is, and fills in what is not already set.
///
/// # Safety
///
/// Call this before the runtime starts. It writes to the process environment,
/// which is only sound while nothing else is running.
pub unsafe fn load() -> Loaded {
    let mut found = candidates();
    let path = if found.is_empty() {
        None
    } else {
        Some(found.remove(0))
    };

    let mut loaded = Loaded {
        path: path.clone(),
        ignored: found,
        applied: Vec::new(),
        overridden: Vec::new(),
        unknown: Vec::new(),
    };

    // Set before the file is read, so the file can still override it and the
    // environment still overrides both.
    if std::env::var_os("PORT").is_none() {
        // SAFETY: the caller promises nothing else is running yet.
        unsafe { std::env::set_var("PORT", DEFAULT_PORT) };
    }

    let Some(path) = path else {
        return loaded;
    };

    let Ok(text) = std::fs::read_to_string(&path) else {
        return loaded;
    };

    for (key, value) in parse(&text) {
        if !KEYS.contains(&key.as_str()) {
            loaded.unknown.push(key);
            continue;
        }

        // The environment wins. Docker and Fly pass these in, and a file that
        // could quietly overrule them would change a deployment by being
        // copied into place. The port is the exception: what is there is this
        // program's own default, not somebody's decision.
        let ours = key == "PORT" && std::env::var("PORT").as_deref() == Ok(DEFAULT_PORT);
        if !ours && std::env::var_os(&key).is_some() {
            loaded.overridden.push(key);
            continue;
        }

        // SAFETY: the caller promises nothing else is running yet.
        unsafe { std::env::set_var(&key, value) };
        loaded.applied.push(key);
    }

    loaded
}

/// Every settings file there is, in the order they are preferred.
fn candidates() -> Vec<PathBuf> {
    // Named outright: no searching, and no surprise about which one won.
    if let Some(named) = std::env::var_os("FUGANTT_CONF") {
        let path = PathBuf::from(named);
        return if path.is_file() {
            vec![path]
        } else {
            Vec::new()
        };
    }

    let mut places: Vec<PathBuf> = Vec::new();

    // Where the person is standing.
    if let Ok(here) = std::env::current_dir() {
        places.push(here);
    }

    // Where their data lives. This is the one that survives an update: a
    // package manager puts the executable in a folder named after the version
    // and throws it away on the next one, taking anything beside it.
    if let Some(dir) = crate::db::data_dir() {
        places.push(dir);
    }

    // Beside the executable, for a copy somebody unzipped and double-clicked.
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        places.push(dir.to_path_buf());
    }

    let mut found: Vec<PathBuf> = Vec::new();
    for place in places {
        for name in NAMES {
            let path = place.join(name);
            if path.is_file() && !found.contains(&path) {
                found.push(path);
            }
        }
    }

    found
}

/// `KEY = value` a line, `#` or `;` starts a comment.
///
/// Deliberately not TOML. There are six settings; a format with sections and
/// arrays would be more grammar than the thing it configures, and one of those
/// settings is a Windows path full of backslashes that no escaping rule should
/// be allowed near.
fn parse(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(|line| line.trim_start_matches('\u{feff}').trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with(';'))
        .filter_map(|line| {
            // `export PORT=3100` is what a shell person will paste in.
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (key, value) = line.split_once('=')?;

            let value = value.trim();
            // Quotes are stripped, and nothing inside them is interpreted:
            // `D:\plans\fugantt.db` has to arrive as it was written.
            let value = value
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .or_else(|| {
                    value
                        .strip_prefix('\'')
                        .and_then(|rest| rest.strip_suffix('\''))
                })
                .unwrap_or(value);

            Some((key.trim().to_ascii_uppercase(), value.to_owned()))
        })
        .collect()
}

/// What to print when somebody asks where the settings come from.
pub fn explain(loaded: &Loaded) -> String {
    let mut out = String::new();

    match &loaded.path {
        Some(path) => out.push_str(&format!("設定ファイル: {}\n", path.display())),
        None => {
            out.push_str("設定ファイルはありません。次の場所を見ます:\n");
            for name in NAMES {
                out.push_str(&format!(
                    "  {name}（カレント / 利用者ごとの場所 / 実行ファイルの隣）\n"
                ));
            }
        }
    }

    for path in &loaded.ignored {
        out.push_str(&format!("  （読んでいません: {}）\n", path.display()));
    }
    if !loaded.applied.is_empty() {
        out.push_str(&format!("  読んだ: {}\n", loaded.applied.join(", ")));
    }
    if !loaded.overridden.is_empty() {
        out.push_str(&format!(
            "  環境変数が優先: {}\n",
            loaded.overridden.join(", ")
        ));
    }
    for key in &loaded.unknown {
        out.push_str(&format!("  分からない項目: {key}\n"));
    }

    out.push_str("\nいまの値:\n");
    for key in KEYS {
        let value = std::env::var(key).unwrap_or_else(|_| "（未設定）".to_owned());
        // A password would not be here, but a token might be one day.
        out.push_str(&format!("  {key} = {value}\n"));
    }

    out
}

/// The text `--help` prints.
pub fn help() -> String {
    let mut out = String::from(concat!(
        "fugantt ",
        env!("CARGO_PKG_VERSION"),
        " — 予定と実施を並べるガントチャート\n\n",
        "起動すると画面が開きます。設定は環境変数か、実行ファイルの隣に置いた\n",
        "fugantt.ini（fugantt.conf・.env でも読みます）に書きます。\n\n",
        "  fugantt            そのまま起動する\n",
        "  fugantt --config   どの設定がどこから来ているかを出す\n",
        "  fugantt --make-admin <ユーザー名>\n",
        "                     その人を管理者にする（管理者が入れなくなったとき）\n",
        "  fugantt --help     これ\n\n",
        "書ける項目:\n",
    ));

    for (key, about) in [
        (
            "HOST",
            "待ち受けるアドレス。既定 127.0.0.1、0.0.0.0 で LAN に公開",
        ),
        ("PORT", "待ち受けるポート。既定 1861（ガントの生年）"),
        (
            "FUGANTT_DB",
            "データベースのファイル。既定は利用者ごとの場所",
        ),
        ("FUGANTT_OPEN", "起動時に画面を開く。window / tab / 0"),
        (
            "FUGANTT_ALLOW_HTTP",
            "1 で平文 HTTP を許可（LAN 用・トークンが平文で流れます）",
        ),
        (
            "FUGANTT_NO_AUTH",
            "認証なしで動かす合言葉。届く人すべてが読み書きできます",
        ),
    ] {
        out.push_str(&format!("  {key:<20} {about}\n"));
    }

    out.push_str("\n書き方:\n  PORT = 3100\n  FUGANTT_DB = D:\\plans\\fugantt.db\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shapes a person actually types, including the ones a text editor
    /// adds without asking.
    #[test]
    fn a_settings_file_reads_the_way_it_looks() {
        let text = concat!(
            "\u{feff}# 社内用\r\n",
            "PORT = 3100\r\n",
            "\r\n",
            "; 旧式のコメント\n",
            "  FUGANTT_DB=D:\\plans\\fugantt.db  \n",
            "export HOST = \"0.0.0.0\"\n",
            "fugantt_open = 'tab'\n",
        );

        assert_eq!(
            parse(text),
            vec![
                ("PORT".to_owned(), "3100".to_owned()),
                // A Windows path arrives exactly as written: no escaping rule
                // is allowed near it.
                ("FUGANTT_DB".to_owned(), "D:\\plans\\fugantt.db".to_owned()),
                ("HOST".to_owned(), "0.0.0.0".to_owned()),
                // Lower case is the same setting; there is only one vocabulary.
                ("FUGANTT_OPEN".to_owned(), "tab".to_owned()),
            ]
        );
    }

    /// A line with no `=` is not a setting, and is not worth stopping over.
    #[test]
    fn nonsense_lines_are_skipped_rather_than_fatal() {
        assert_eq!(
            parse("PORT\nHOST=1\n"),
            vec![("HOST".to_owned(), "1".to_owned())]
        );
    }

    /// An empty value clears nothing and sets nothing surprising.
    #[test]
    fn an_empty_value_is_an_empty_value() {
        assert_eq!(
            parse("FUGANTT_OPEN=\n"),
            vec![("FUGANTT_OPEN".to_owned(), String::new())]
        );
    }

    /// Every key the help text lists is a key the loader will accept.
    #[test]
    fn the_help_and_the_loader_agree() {
        let text = help();
        for key in KEYS {
            assert!(text.contains(key), "{key} が --help に出ていない");
        }
    }
}

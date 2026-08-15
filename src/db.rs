use std::{
    error::Error,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use topcoat::context::{Cx, app_context};

/// The connection pool, registered as app context so every handler shares it.
pub struct Db(pub SqlitePool);

pub fn pool(cx: &Cx) -> &SqlitePool {
    &app_context::<Db>(cx).0
}

/// Opens the database, creating and migrating it when needed.
pub async fn connect(path: &str) -> Result<SqlitePool, Box<dyn Error>> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        // WAL keeps readers off the single writer's back.
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;

    sqlx::migrate!().run(&pool).await?;
    ensure_an_admin(&pool).await?;

    Ok(pool)
}

/// Warns when the installation has nobody in it.
///
/// The register form is open while the users table is empty — that is how the
/// first administrator gets in — and a window nobody knows about is the kind
/// that stays open.
async fn ensure_an_admin(pool: &SqlitePool) -> Result<(), Box<dyn Error>> {
    let (admins,) =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM users WHERE base_role = 'admin'")
            .fetch_one(pool)
            .await?;

    if admins == 0 {
        eprintln!(
            "管理者がいません。/login で最初のアカウントを作ってください（作った人が管理者になります）。"
        );
    }

    Ok(())
}

/// The current time as unix seconds, the encoding every timestamp column uses.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Converts a session expiry into the same encoding.
pub fn unix_seconds(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// The database this process is using, remembered as it was resolved.
///
/// Backups are written next to it, and by then the answer must be the same one
/// the pool was opened with — not whatever `path()` would say now.
static FILE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Records the file `connect` was pointed at.
pub fn remember(path: &std::path::Path) {
    let _ = FILE.set(path.to_path_buf());
}

pub fn file() -> std::path::PathBuf {
    FILE.get().cloned().unwrap_or_else(path)
}

/// Where the database file is, and why.
///
/// Three answers in order, because the wrong one loses somebody's data:
///
/// 1. `FUGANTT_DB`, when it is set. Docker and Fly set it, so nothing below
///    can move a deployed database.
/// 2. `fugantt.db` in the working directory, when one is already there. This
///    used to be the only answer, and replacing the executable must never
///    leave a person staring at an empty schedule with their plan still on
///    disk a metre away.
/// 3. The per-user data directory, for a fresh install. A tool on `PATH` gets
///    run from wherever the prompt happens to be, and a database per prompt is
///    a database nobody can find twice.
pub fn path() -> std::path::PathBuf {
    if let Ok(chosen) = std::env::var("FUGANTT_DB") {
        return chosen.into();
    }

    let beside_me = std::path::PathBuf::from(FILE_NAME);
    if beside_me.exists() {
        return beside_me;
    }

    match data_dir() {
        Some(dir) => {
            // A directory that cannot be made is not a place to put a database;
            // fall back rather than fail, and say where it went.
            if std::fs::create_dir_all(&dir).is_ok() {
                dir.join(FILE_NAME)
            } else {
                beside_me
            }
        }
        None => beside_me,
    }
}

const FILE_NAME: &str = "fugantt.db";

/// The per-user place for application data, by the platform's own convention.
fn data_dir() -> Option<std::path::PathBuf> {
    let home = || std::env::var_os("HOME").map(std::path::PathBuf::from);

    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .map(|dir| dir.join("fugantt"))
    } else if cfg!(target_os = "macos") {
        home().map(|dir| dir.join("Library/Application Support/fugantt"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| home().map(|dir| dir.join(".local/share")))
            .map(|dir| dir.join("fugantt"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An explicit setting wins, because Docker and Fly set it and a default
    /// that could move a deployed database is not a default at all.
    #[test]
    fn the_environment_has_the_last_word() {
        temporarily("FUGANTT_DB", Some("/srv/plans.db"), || {
            assert_eq!(path(), std::path::PathBuf::from("/srv/plans.db"));
        });
    }

    /// A database already sitting in the working directory is the one in use,
    /// whatever the default has since become. Replacing the executable must not
    /// hand somebody an empty schedule with their plan still on disk.
    #[test]
    fn a_database_already_here_keeps_being_the_one() {
        let dir = std::env::temp_dir().join(format!("fugantt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE_NAME), b"").unwrap();

        // Serialised with the other environment tests by the same lock.
        temporarily("FUGANTT_DB", None, || {
            let here = std::env::current_dir().unwrap();
            std::env::set_current_dir(&dir).unwrap();
            let found = path();
            std::env::set_current_dir(here).unwrap();

            assert_eq!(found, std::path::PathBuf::from(FILE_NAME));
        });

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A fresh install lands in the platform's own place for user data. A tool
    /// on `PATH` is run from wherever the prompt happens to be, and a database
    /// per prompt is a database nobody finds twice.
    #[test]
    fn a_fresh_install_uses_the_user_data_directory() {
        let dir = data_dir().expect("この環境には HOME か LOCALAPPDATA がある");

        assert!(dir.ends_with("fugantt"), "{}", dir.display());
        assert!(dir.is_absolute(), "{}", dir.display());
    }

    /// Environment variables are process-wide, so the tests that lean on them
    /// take turns.
    fn temporarily(key: &str, value: Option<&str>, run: impl FnOnce()) {
        use std::sync::{Mutex, OnceLock};
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        let guard = LOCK.get_or_init(|| Mutex::new(())).lock();
        let restore = std::env::var(key).ok();

        // SAFETY: single-threaded within the lock above; nothing else in this
        // binary reads the environment while a test holds it.
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }

        run();

        unsafe {
            match restore {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }

        drop(guard);
    }
}

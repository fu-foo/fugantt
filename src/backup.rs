//! Making a copy of the database, and putting one back.
//!
//! `sqlite3 fugantt.db "VACUUM INTO …"` is in the README, and it is the right
//! command for whoever already knows what a shell is. Everyone else is running
//! one executable that they were told needs nothing else, and telling them
//! their data is only safe if they learn another tool takes the promise back.
//!
//! `VACUUM INTO` rather than copying the file: with WAL on, the file on disk is
//! only part of the database, and a copy of it is a plausible-looking ruin.

use std::path::{Path, PathBuf};

use sqlx::{Connection, Executor, Row, SqlitePool};
use topcoat::{Result, context::Cx, router::error::bad_request};

use crate::db;

/// Every SQLite file starts with this, so a wrong file can be turned away
/// before it is written anywhere.
const MAGIC: &[u8] = b"SQLite format 3\0";

/// The tables that must be there for this to be one of ours.
///
/// All of these have existed since the first migration, so an old backup still
/// passes — which is the point, since an old backup is the one you reach for.
/// `_sqlx_migrations` is in the list because it says how far the file has been
/// brought: without it, migrating would try to create tables that are already
/// there and fail with something nobody can act on.
const OURS: [&str; 4] = ["projects", "tasks", "users", "_sqlx_migrations"];

/// A consistent copy of the database, as bytes to hand to a browser.
pub async fn snapshot(cx: &Cx) -> Result<Vec<u8>> {
    let path = scratch("fugantt-backup");

    copy_into(db::pool(cx), &path).await?;
    let bytes = std::fs::read(&path).map_err(|error| bad_request(error.to_string()))?;
    let _ = std::fs::remove_file(&path);

    Ok(bytes)
}

/// Replaces everything in the live database with the contents of `bytes`.
///
/// Returns where the previous contents were put. Somebody restoring a backup
/// is having a bad enough day without the tool making the old state
/// unreachable the moment they act — and "restore the wrong file" is a mistake
/// that happens precisely when people are in a hurry.
pub async fn restore(cx: &Cx, bytes: &[u8], l: crate::i18n::Lang) -> Result<PathBuf> {
    if !bytes.starts_with(MAGIC) {
        return Err(bad_request(l.t("SQLite のファイルではありません。")).into());
    }

    let incoming = scratch("fugantt-restore");
    std::fs::write(&incoming, bytes).map_err(|error| bad_request(error.to_string()))?;

    let result = replace_with(cx, &incoming, l).await;

    // The upload is a copy of a copy; nothing is lost by clearing it away, and
    // leaving whole databases in the temporary directory is its own problem.
    let _ = std::fs::remove_file(&incoming);
    for extra in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{extra}", incoming.display()));
    }

    result
}

async fn replace_with(cx: &Cx, incoming: &Path, l: crate::i18n::Lang) -> Result<PathBuf> {
    // Opened and checked before anything is touched. `db::connect` would create
    // and migrate whatever it was pointed at, which would turn a wrong file
    // into an empty valid one and restore *that*.
    let source = SqlitePool::connect(&format!("sqlite://{}", incoming.display()))
        .await
        .map_err(|_| bad_request(l.t("ファイルを開けませんでした。")))?;

    for table in OURS {
        let (found,) = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        )
        .bind(table)
        .fetch_one(&source)
        .await
        .map_err(|error| bad_request(error.to_string()))?;

        if found == 0 {
            source.close().await;
            return Err(bad_request(l.t("fugantt のバックアップではありません。")).into());
        }
    }

    // Brought up to date, so a backup from an older version can still be put
    // back. This is the whole reason the file is opened rather than swapped in:
    // the copy has to match the schema the running code expects.
    sqlx::migrate!()
        .run(&source)
        .await
        .map_err(|error| bad_request(error.to_string()))?;
    source.close().await;

    let kept = beside_the_database(&format!(
        "fugantt-before-restore-{}.db",
        jiff::Zoned::now().strftime("%Y%m%d-%H%M%S")
    ));
    copy_into(db::pool(cx), &kept).await?;

    let pool = db::pool(cx);
    let mut connection = pool
        .acquire()
        .await
        .map_err(|error| bad_request(error.to_string()))?;

    // Rows arrive in whatever order the tables are listed, and half of them
    // point at each other. The check is off for the swap and on again after —
    // outside the transaction, because SQLite ignores the pragma inside one.
    connection
        .execute("PRAGMA foreign_keys = OFF")
        .await
        .map_err(|error| bad_request(error.to_string()))?;

    let outcome = swap(&mut connection, incoming).await;

    let _ = connection.execute("DETACH DATABASE incoming").await;
    let _ = connection.execute("PRAGMA foreign_keys = ON").await;

    outcome?;

    Ok(kept)
}

/// The swap itself: one transaction, so a failure leaves the plan as it was.
async fn swap(connection: &mut sqlx::SqliteConnection, incoming: &Path) -> Result<()> {
    let attach = format!("ATTACH DATABASE '{}' AS incoming", quote(incoming));
    connection
        .execute(attach.as_str())
        .await
        .map_err(|error| bad_request(error.to_string()))?;

    let tables: Vec<String> = sqlx::query(
        "SELECT name FROM main.sqlite_master
          WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name <> '_sqlx_migrations'
          ORDER BY name",
    )
    .fetch_all(&mut *connection)
    .await
    .map_err(|error| bad_request(error.to_string()))?
    .into_iter()
    .map(|row| row.get::<String, _>("name"))
    .collect();

    let mut transaction = connection
        .begin()
        .await
        .map_err(|error| bad_request(error.to_string()))?;

    for table in &tables {
        // Named columns rather than `SELECT *`: both files have been through the
        // same migrations, but a column added by one and rebuilt by another can
        // still leave the order different, and lining up 予定開始 with 進捗 is
        // the kind of failure that looks like data rather than an error.
        let columns: Vec<String> = sqlx::query(&format!(
            "SELECT ti.name FROM pragma_table_info('{table}') AS ti
              WHERE ti.name IN (SELECT name FROM pragma_table_info('{table}'))"
        ))
        .fetch_all(&mut *transaction)
        .await
        .map_err(|error| bad_request(error.to_string()))?
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

        let list = columns
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ");

        transaction
            .execute(format!("DELETE FROM main.\"{table}\"").as_str())
            .await
            .map_err(|error| bad_request(error.to_string()))?;

        // A table the backup does not have is simply left empty: an older file
        // knows nothing about a feature added since, and that is not an error.
        let exists = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM incoming.sqlite_master WHERE type = 'table' AND name = ?1",
        )
        .bind(table)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| bad_request(error.to_string()))?;

        if exists.0 == 0 {
            continue;
        }

        transaction
            .execute(
                format!(
                    "INSERT INTO main.\"{table}\" ({list}) SELECT {list} FROM incoming.\"{table}\""
                )
                .as_str(),
            )
            .await
            .map_err(|error| bad_request(error.to_string()))?;
    }

    transaction
        .commit()
        .await
        .map_err(|error| bad_request(error.to_string()))?;

    Ok(())
}

/// `VACUUM INTO`, which is the only copy of a WAL database worth keeping.
async fn copy_into(pool: &SqlitePool, path: &Path) -> Result<()> {
    // A leftover from a previous attempt would make this fail; the file is ours
    // either way, named after this installation and this second.
    let _ = std::fs::remove_file(path);

    sqlx::query(&format!("VACUUM INTO '{}'", quote(path)))
        .execute(pool)
        .await
        .map_err(|error| bad_request(error.to_string()))?;

    Ok(())
}

/// A path SQLite will read as one string. Only the quote needs doubling.
fn quote(path: &Path) -> String {
    path.display().to_string().replace('\'', "''")
}

/// Somewhere to put a file that is on its way somewhere else.
fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}-{}.db", uuid::Uuid::new_v4()))
}

/// Next to the live database, which is the one place we know is writable and
/// the one place somebody will think to look.
fn beside_the_database(name: &str) -> PathBuf {
    match db::file().parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.join(name),
        _ => PathBuf::from(name),
    }
}

/// What to call the file a person is about to download.
pub fn filename() -> String {
    format!(
        "fugantt-{}.db",
        jiff::Zoned::now().strftime("%Y%m%d-%H%M%S")
    )
}

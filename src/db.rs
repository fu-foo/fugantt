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
    let (admins,) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM users WHERE base_role = 'admin'")
        .fetch_one(pool)
        .await?;

    if admins == 0 {
        eprintln!("管理者がいません。/login で最初のアカウントを作ってください（作った人が管理者になります）。");
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
    time.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

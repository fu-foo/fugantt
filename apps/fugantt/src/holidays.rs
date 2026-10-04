//! Keeping Japan's public holidays in the installation's calendar.
//!
//! Which days they are is `fu_calendar`'s business. This is the part that
//! writes them down, once a year, without being asked.

pub use fu_calendar::japanese;
use jiff::civil::Date;

/// The setting that lists which years' holidays have been put in.
const FILLED: &str = "japan_holiday_years";

/// Keeps last year, this year and next year's Japanese holidays in the
/// installation's calendar, without being asked.
///
/// A new installation started with no holidays at all, so every 祝日 counted
/// as a working day — in the day counts, the lateness, 空き検索 — until an
/// administrator found the button; and the next January, again. This runs at
/// start and then once a day.
///
/// Each year is put in once and then left alone. A company that works on 海の日
/// deletes it, and it stays deleted: the year is already on the list, so it is
/// never filled again. A year that already has any holiday in it when this
/// first sees it counts as filled too — somebody has been keeping it by hand,
/// and their removals are the ones to keep.
///
/// Returns whether anything was added, so the plans can be told to redraw.
pub async fn keep_filled(pool: &sqlx::SqlitePool, today: Date) -> Result<bool, sqlx::Error> {
    let stored: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = ?1")
        .bind(FILLED)
        .fetch_optional(pool)
        .await?;
    let mut filled: Vec<i16> = stored
        .map(|(value,)| {
            value
                .split_whitespace()
                .filter_map(|year| year.parse().ok())
                .collect()
        })
        .unwrap_or_default();

    let mut added = false;

    for year in [today.year() - 1, today.year(), today.year() + 1] {
        if filled.contains(&year) || !(2020..=2099).contains(&year) {
            continue;
        }
        filled.push(year);

        let (kept,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM app_holidays WHERE date LIKE ?1")
                .bind(format!("{year:04}-%"))
                .fetch_one(pool)
                .await?;
        if kept > 0 {
            continue;
        }

        for (date, name) in japanese(year) {
            sqlx::query(
                "INSERT INTO app_holidays (date, name) VALUES (?1, ?2) ON CONFLICT (date) DO NOTHING",
            )
            .bind(date.to_string())
            .bind(name)
            .execute(pool)
            .await?;
        }
        added = true;
    }

    filled.sort_unstable();
    let list = filled
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )
    .bind(FILLED)
    .bind(list)
    .execute(pool)
    .await?;

    Ok(added)
}

#[cfg(test)]
mod tests {
    async fn calendar() -> (sqlx::SqlitePool, std::path::PathBuf) {
        let path =
            std::env::temp_dir().join(format!("fugantt-holidays-{}.db", uuid::Uuid::new_v4()));
        let pool = crate::db::connect(&path.to_string_lossy()).await.unwrap();
        (pool, path)
    }

    async fn days(pool: &sqlx::SqlitePool, year: i16) -> i64 {
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM app_holidays WHERE date LIKE ?1")
            .bind(format!("{year:04}-%"))
            .fetch_one(pool)
            .await
            .unwrap()
            .0
    }

    #[tokio::test]
    async fn a_new_installation_has_its_holidays_without_asking() {
        let (pool, path) = calendar().await;
        let today: Date = "2026-10-02".parse().unwrap();

        assert!(keep_filled(&pool, today).await.unwrap());
        for year in [2025, 2026, 2027] {
            assert_eq!(
                days(&pool, year).await,
                japanese(year).len() as i64,
                "{year}"
            );
        }

        // Asked again the same day, nothing more to do.
        assert!(!keep_filled(&pool, today).await.unwrap());

        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn a_holiday_taken_out_stays_out() {
        let (pool, path) = calendar().await;
        let today: Date = "2026-10-02".parse().unwrap();
        keep_filled(&pool, today).await.unwrap();

        // 海の日 is a working day here.
        sqlx::query("DELETE FROM app_holidays WHERE date = '2026-07-20'")
            .execute(&pool)
            .await
            .unwrap();
        keep_filled(&pool, today).await.unwrap();

        let back: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM app_holidays WHERE date = '2026-07-20'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(back.0, 0);

        // A new year arrives with the calendar: next January, 2028 is filled in.
        assert!(
            keep_filled(&pool, "2027-01-01".parse().unwrap())
                .await
                .unwrap()
        );
        assert!(days(&pool, 2028).await > 0);

        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn a_year_somebody_kept_by_hand_is_left_to_them() {
        let (pool, path) = calendar().await;
        sqlx::query("INSERT INTO app_holidays (date, name) VALUES ('2026-08-13', '夏季休業')")
            .execute(&pool)
            .await
            .unwrap();

        keep_filled(&pool, "2026-10-02".parse().unwrap())
            .await
            .unwrap();

        assert_eq!(days(&pool, 2026).await, 1, "手で入れた年には足さない");
        assert!(days(&pool, 2027).await > 0);

        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    use super::*;
}

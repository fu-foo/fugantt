//! Slowing down guessing at the login form.
//!
//! Argon2 already makes each attempt expensive, which cuts both ways: it is
//! also what a flood of attempts would exhaust the machine with. Counting
//! failures and refusing for a while costs nothing and stops both.
//!
//! Held in memory rather than in the database: the counters are worth losing on
//! restart, and a restart is not something an attacker can cause.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use topcoat::context::{Cx, app_context};

/// Failures allowed before the door closes.
const LIMIT: u32 = 8;
/// How long failures are remembered.
const WINDOW: Duration = Duration::from_secs(15 * 60);
/// How long the door stays closed once it does.
const COOLDOWN: Duration = Duration::from_secs(15 * 60);

#[derive(Debug)]
struct Record {
    failures: u32,
    since: Instant,
}

#[derive(Default)]
pub struct Attempts {
    records: Mutex<HashMap<String, Record>>,
}

impl Attempts {
    /// How long the caller must wait, or `None` when it may proceed.
    pub fn retry_after(&self, keys: &[String]) -> Option<Duration> {
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);

        keys.iter()
            .filter_map(|key| {
                let record = records.get(key)?;

                if record.failures < LIMIT {
                    return None;
                }

                COOLDOWN.checked_sub(record.since.elapsed())
            })
            .max()
            .inspect(|_| {
                // Expired entries would otherwise pile up for as long as the
                // process runs.
                records.retain(|_, record| record.since.elapsed() < WINDOW.max(COOLDOWN));
            })
    }

    pub fn record_failure(&self, keys: &[String]) {
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);

        for key in keys {
            let record = records.entry(key.clone()).or_insert(Record {
                failures: 0,
                since: Instant::now(),
            });

            // A quiet stretch starts the count over; a run of failures does not.
            if record.since.elapsed() > WINDOW {
                record.failures = 0;
                record.since = Instant::now();
            }

            record.failures += 1;
        }
    }

    /// A successful sign-in clears the slate for those keys.
    pub fn forget(&self, keys: &[String]) {
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);

        for key in keys {
            records.remove(key);
        }
    }
}

pub fn attempts(cx: &Cx) -> &Attempts {
    app_context(cx)
}

/// What to count a login attempt against.
///
/// The email is always there. The address only exists behind a proxy that
/// passes it on — on fly.io or nginx it will, on a bare LAN it will not, and
/// the email key carries the load on its own.
pub fn keys(cx: &Cx, email: &str) -> Vec<String> {
    let mut keys = vec![format!("email:{email}")];

    if let Some(ip) = client_ip(cx) {
        keys.push(format!("ip:{ip}"));
    }

    keys
}

fn client_ip(cx: &Cx) -> Option<String> {
    let headers = topcoat::router::headers(cx);

    // `Fly-Client-IP` first: on fly.io it is the one the platform sets and a
    // client cannot forge. `X-Forwarded-For` may be a chain; the first entry is
    // the original client as the nearest proxy saw it.
    for name in ["fly-client-ip", "x-forwarded-for", "x-real-ip"] {
        if let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) {
            let first = value.split(',').next().unwrap_or(value).trim();

            if !first.is_empty() {
                return Some(first.to_owned());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> Vec<String> {
        vec!["email:a@example.com".to_owned()]
    }

    #[test]
    fn the_door_closes_after_enough_failures() {
        let attempts = Attempts::default();

        for _ in 0..LIMIT - 1 {
            attempts.record_failure(&keys());
        }
        assert!(attempts.retry_after(&keys()).is_none(), "まだ開いているはず");

        attempts.record_failure(&keys());
        assert!(attempts.retry_after(&keys()).is_some(), "閉じるはず");
    }

    /// Signing in successfully has to clear the count, or one forgotten
    /// password would lock someone out for the rest of the window.
    #[test]
    fn a_successful_sign_in_clears_it() {
        let attempts = Attempts::default();

        for _ in 0..LIMIT {
            attempts.record_failure(&keys());
        }
        assert!(attempts.retry_after(&keys()).is_some());

        attempts.forget(&keys());
        assert!(attempts.retry_after(&keys()).is_none());
    }

    #[test]
    fn one_key_closing_does_not_close_another() {
        let attempts = Attempts::default();

        for _ in 0..LIMIT {
            attempts.record_failure(&keys());
        }

        assert!(attempts.retry_after(&["email:b@example.com".to_owned()]).is_none());
    }
}

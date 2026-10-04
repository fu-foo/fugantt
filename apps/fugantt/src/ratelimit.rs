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

/// Past this many, the stale records are swept on the next failure.
const MAX_RECORDS: usize = 10_000;

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

        // A record is kept for every name anybody tried. Cleared only when a
        // door closed, a stream of different names grew this without end.
        if records.len() > MAX_RECORDS {
            records.retain(|_, record| record.since.elapsed() < WINDOW.max(COOLDOWN));
        }

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
/// The email is always there. The address only where it can be trusted — on
/// fly.io — and elsewhere the email key carries the load on its own.
pub fn keys(cx: &Cx, email: &str) -> Vec<String> {
    let mut keys = vec![format!("email:{email}")];

    if let Some(ip) = client_ip(cx) {
        keys.push(format!("ip:{ip}"));
    }

    keys
}

/// The address the request came from — when anyone can vouch for it.
///
/// A forwarding header is whatever the sender wrote. Read from a request that
/// came straight to this server, `X-Forwarded-For: 1.2.3.4` is a new address on
/// every attempt, and the limit by address is no limit. Fly's edge overwrites
/// `Fly-Client-IP` itself, so there it means what it says; anywhere else the
/// limit is by account alone. The framework does not hand the socket's own
/// address to a request, or that would be the answer here.
fn client_ip(cx: &Cx) -> Option<String> {
    std::env::var_os("FLY_APP_NAME")?;

    let headers = topcoat::router::headers(cx);
    let value = headers.get("fly-client-ip")?.to_str().ok()?.trim();

    (!value.is_empty()).then(|| value.to_owned())
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
        assert!(
            attempts.retry_after(&keys()).is_none(),
            "まだ開いているはず"
        );

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

        assert!(
            attempts
                .retry_after(&["email:b@example.com".to_owned()])
                .is_none()
        );
    }
}

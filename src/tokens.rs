//! Access tokens, for the things that are not a browser.
//!
//! A token belongs to one project and carries one role. It is shown once, at
//! the moment it is made, and only its hash is kept — a copy of the database is
//! a list of hashes rather than a ring of working keys.
//!
//! The same machinery as a session token, because the problem is the same one:
//! 32 bytes of randomness that must never be readable back out of storage.

use topcoat::{Result, context::Cx, session::Token as Random};

use crate::db;

/// The prefix makes a leaked token recognisable in a log or a paste.
const PREFIX: &str = "fug_";

#[derive(Debug, sqlx::FromRow)]
pub struct Token {
    pub id: String,
    pub name: String,
    pub role: String,
    pub last_used: Option<i64>,
}

/// A fresh token: the part to show once, and the part to store.
pub fn generate() -> (String, Vec<u8>) {
    let random = Random::random();

    (format!("{PREFIX}{}", random.encode()), random.hash().to_vec())
}

/// The hash a presented token would be stored under.
pub fn digest(token: &str) -> Option<Vec<u8>> {
    let body = token.strip_prefix(PREFIX)?;

    Random::decode(body).ok().map(|token| token.hash().to_vec())
}

/// The project and role a token opens, or nothing.
///
/// Records that it was used on the way past: a list of tokens nobody can date
/// is a list nobody dares revoke.
pub async fn resolve(cx: &Cx, token: &str) -> Result<Option<(String, String)>> {
    let Some(hash) = digest(token) else {
        return Ok(None);
    };

    let found = sqlx::query_as::<_, (String, String)>(
        "SELECT project_id, role FROM api_tokens WHERE token_hash = ?1",
    )
    .bind(&hash[..])
    .fetch_optional(db::pool(cx))
    .await?;

    if found.is_some() {
        sqlx::query("UPDATE api_tokens SET last_used = ?2 WHERE token_hash = ?1")
            .bind(&hash[..])
            .bind(db::now())
            .execute(db::pool(cx))
            .await?;
    }

    Ok(found)
}

/// The tokens a project has, newest first. Never the tokens themselves.
pub async fn list(cx: &Cx, project_id: &str) -> Result<Vec<Token>> {
    Ok(sqlx::query_as::<_, Token>(
        "SELECT id, name, role, last_used
           FROM api_tokens
          WHERE project_id = ?1
          ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(db::pool(cx))
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_recognisable_and_hashes_back_to_itself() {
        let (token, hash) = generate();

        assert!(token.starts_with(PREFIX));
        assert_eq!(digest(&token), Some(hash));
    }

    #[test]
    fn anything_else_is_not_a_token() {
        assert_eq!(digest("hunter2"), None);
        assert_eq!(digest("fug_not-base64!!"), None);
    }
}

//! Running without accounts.
//!
//! On a closed network the sign-in screen can be more friction than
//! protection: everyone who can reach the machine is already meant to use it.
//! `FUGANTT_NO_AUTH` drops the wall entirely — every visitor becomes the same
//! shared user, with no way to tell them apart.
//!
//! It is deliberately awkward to switch on. Anyone who reaches it should have
//! read what it does, so it takes a phrase rather than a flag, refuses to
//! combine with anything reachable from outside, and says so on every page.

use topcoat::{Result, context::Cx};

use crate::db;

/// The phrase the variable has to carry. A `1` is too easy to type by habit.
const CONFIRMATION: &str = "yes-everyone-on-this-network-can-edit";

/// The account every visitor shares when authentication is off.
pub const SHARED_EMAIL: &str = "共有アカウント";
const SHARED_ID: &str = "shared-account";

/// Whether the server was started without authentication.
pub fn enabled() -> bool {
    std::env::var("FUGANTT_NO_AUTH").is_ok_and(|value| value == CONFIRMATION)
}

/// Complains loudly, and refuses the combinations that would be reckless.
///
/// Returns the message to print, or an error that should stop the server.
pub fn check() -> Result<Option<String>, String> {
    let raw = std::env::var("FUGANTT_NO_AUTH").unwrap_or_default();

    if raw.is_empty() {
        return Ok(None);
    }

    if raw != CONFIRMATION {
        return Err(format!(
            "FUGANTT_NO_AUTH の値が違います。\n\
             認証なしで動かすと、この URL に届く人は全員が全プロジェクトを読み書きできます。\n\
             それでよければ次の値を設定してください:\n\n    \
             FUGANTT_NO_AUTH={CONFIRMATION}"
        ));
    }

    // Reachable from beyond the machine is the case where this matters, and the
    // operator should have said so twice.
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let exposed = host != "127.0.0.1" && host != "localhost";

    if exposed && std::env::var("FUGANTT_NO_AUTH_I_MEAN_IT").is_err() {
        return Err(format!(
            "HOST={host} で認証なしは、ネットワーク越しの全員に開放することを意味します。\n\
             本当にそうしたい場合のみ、もう一つ設定してください:\n\n    \
             FUGANTT_NO_AUTH_I_MEAN_IT=1"
        ));
    }

    Ok(Some(format!(
        "認証なしで起動しています。{}に届く人は全員が全プロジェクトを読み書きできます。",
        if exposed {
            "このネットワーク"
        } else {
            "この端末"
        }
    )))
}

/// Makes sure the shared account exists, so everything downstream can assume a
/// user the way it always has.
pub async fn ensure_shared_user(cx: &Cx) -> Result<()> {
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, created_at, base_role)
         VALUES (?1, ?2, '', ?3, 'admin')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(SHARED_ID)
    .bind(SHARED_EMAIL)
    .bind(db::now())
    .execute(db::pool(cx))
    .await?;

    Ok(())
}

/// The shared user, created on first use.
///
/// The password hash is empty, which no Argon2 verification can ever match, so
/// the account cannot be signed into even if authentication is turned back on.
pub async fn shared_user(cx: &Cx) -> Result<crate::auth::User> {
    ensure_shared_user(cx).await?;

    Ok(crate::auth::User {
        id: SHARED_ID.to_owned(),
        email: SHARED_EMAIL.to_owned(),
        display_name: "みんな".to_owned(),
        base_role: "admin".to_owned(),
        language: String::new(),
        // Without sign-in there is nobody to have a preference: everyone
        // shares the account, so a theme set here would be set for the room.
        theme: String::new(),
        custom_css: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The phrase is the point: a `1` set by habit must not open the door.
    #[test]
    fn only_the_full_phrase_counts() {
        assert_ne!(CONFIRMATION, "1");
        assert!(CONFIRMATION.len() > 20);
        assert!(CONFIRMATION.contains("everyone"));
    }
}

//! The account list.
//!
//! Accounts are made by whoever runs the installation, not by the people using
//! it: name, username, password and base role in one form. Invitation links
//! were a roundabout way to do the same thing on a network where the
//! administrator can just say "here is your login".

use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::FromRow;
use topcoat::{Result, context::Cx};

use crate::db;

#[derive(Debug, Clone, FromRow)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub base_role: String,
}

impl Account {
    pub fn name(&self) -> &str {
        if self.display_name.is_empty() {
            &self.email
        } else {
            &self.display_name
        }
    }
}

/// What a base role means on a project that does not name the person.
pub const ROLES: [(&str, &str); 4] = [
    ("admin", "管理者（全体）"),
    ("editor", "編集者"),
    ("viewer", "閲覧者"),
    ("none", "無効（招かれたプロジェクトだけ）"),
];

/// What one account is called, for a record that outlives it.
pub async fn one(cx: &Cx, id: &str) -> Result<Option<String>> {
    let name = sqlx::query_scalar::<_, String>(
        "SELECT CASE WHEN display_name = '' THEN email ELSE display_name END
           FROM users WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(db::pool(cx))
    .await?;

    Ok(name)
}

/// Whether nobody has registered yet, which is the one moment the register
/// form is open.
pub async fn none_yet(cx: &Cx) -> Result<bool> {
    let (count,) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM users")
        .fetch_one(db::pool(cx))
        .await?;

    Ok(count == 0)
}

pub async fn list(cx: &Cx) -> Result<Vec<Account>> {
    let accounts = sqlx::query_as::<_, Account>(
        "SELECT id, email, display_name, base_role FROM users ORDER BY created_at",
    )
    .fetch_all(db::pool(cx))
    .await?;

    Ok(accounts)
}

pub async fn by_id(cx: &Cx, id: &str) -> Result<Option<Account>> {
    let account = sqlx::query_as::<_, Account>(
        "SELECT id, email, display_name, base_role FROM users WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(db::pool(cx))
    .await?;

    Ok(account)
}

/// Hashes off the async runtime: Argon2 is deliberately CPU-heavy and would
/// otherwise stall every other request sharing the worker thread.
pub async fn hash_password(password: String) -> Result<String> {
    let hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);

        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| std::io::Error::other(error.to_string()))
    })
    .await??;

    Ok(hash)
}

/// Whether that display name already belongs to somebody else.
///
/// On a task an assignee is a name, so two accounts sharing a display name make
/// their work indistinguishable for good. Names are only ever set here, so this
/// is where it gets refused.
pub async fn name_is_taken(cx: &Cx, name: &str, except_id: &str) -> Result<bool> {
    let name = name.trim();

    if name.is_empty() {
        return Ok(false);
    }

    let (hits,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM users
          WHERE id <> ?2
            AND (CASE WHEN display_name = '' THEN email ELSE display_name END) = ?1",
    )
    .bind(name)
    .bind(except_id)
    .fetch_one(db::pool(cx))
    .await?;

    Ok(hits > 0)
}

/// Renames somebody everywhere their name is written.
///
/// 担当者 is a name, not an id — plans name people who have no account here —
/// so a rename that only touched the account would leave every assignment
/// pointing at somebody who no longer exists.
pub async fn rename_everywhere(cx: &Cx, from: &str, to: &str) -> Result<()> {
    if from == to || from.is_empty() {
        return Ok(());
    }

    for statement in [
        "UPDATE tasks SET assignee = ?2 WHERE assignee = ?1",
        "UPDATE leaves SET assignee = ?2 WHERE assignee = ?1",
        // Name is the key in these lists, so an existing target replaces the old row.
        "UPDATE OR REPLACE project_assignees SET name = ?2 WHERE name = ?1",
        "UPDATE OR REPLACE assignees SET name = ?2 WHERE name = ?1",
    ] {
        sqlx::query(statement)
            .bind(from)
            .bind(to)
            .execute(db::pool(cx))
            .await?;
    }

    Ok(())
}

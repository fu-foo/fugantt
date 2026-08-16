use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use serde::Deserialize;
use sqlx::FromRow;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{RouterErrorExt, SeeOther, bad_request, see_other},
        route,
    },
    session,
};

use crate::{db, ratelimit, users};

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: String,
    /// The login name. Not necessarily an address — `yamada` is fine.
    pub email: String,
    /// What this person is called on screen. Empty for accounts made before
    /// names existed, which fall back to the login name.
    pub display_name: String,
    /// What they can do on a project that does not name them: `admin`,
    /// `editor`, `viewer`, or `none`.
    pub base_role: String,
    /// The language of the screen. Empty follows the installation setting.
    pub language: String,
    /// How this person wants the screen to look: `light`, `dark`, or empty to
    /// follow whatever their machine says.
    pub theme: String,
    /// Their own CSS, served to them and to nobody else.
    pub custom_css: String,
}

impl User {
    /// Whether they run the installation: users, invitations, era table.
    pub fn is_admin(&self) -> bool {
        self.base_role == "admin"
    }

    /// The name to show. An email address is neither what anyone is called nor
    /// short enough to sit in a cell.
    pub fn display(&self) -> &str {
        if self.display_name.is_empty() {
            &self.email
        } else {
            &self.display_name
        }
    }
}

/// The signed-in user, or `None` when the request carries no live session.
///
/// The session cookie only proves possession of a token; the `sessions` row is
/// what ties that token to a user, and an expired row is not a login.
pub async fn current_user(cx: &Cx) -> Result<Option<User>> {
    // Without authentication everyone is the same person, and there is no
    // session to consult.
    if crate::open_access::enabled() {
        return Ok(Some(crate::open_access::shared_user(cx).await?));
    }

    let Some(hash) = session::token_hash(cx).await? else {
        return Ok(None);
    };

    let user = sqlx::query_as::<_, User>(
        "SELECT users.id, users.email, users.display_name, users.base_role, users.language,
                users.theme, users.custom_css
           FROM sessions
           JOIN users ON users.id = sessions.user_id
          WHERE sessions.token_hash = ?1
            AND sessions.expires_at > ?2",
    )
    .bind(&hash[..])
    .bind(db::now())
    .fetch_optional(db::pool(cx))
    .await?;

    Ok(user)
}

/// The signed-in user, redirecting to the login page when there is none.
pub async fn require_user(cx: &Cx) -> Result<User> {
    Ok(current_user(cx).await?.ok_or_redirect("/login")?)
}

#[derive(Deserialize)]
pub struct Credentials {
    email: String,
    password: String,
    /// The display name, from the register form only.
    name: Option<String>,
}

#[route(POST "/register")]
async fn register(cx: &Cx, Form(form): Form<Credentials>) -> Result<SeeOther> {
    let email = form.email.trim().to_lowercase();

    // Not necessarily an address: an office that runs this on its own network
    // hands out names like 山田 or yamada, and insisting on an @ would mean
    // inventing addresses that nobody reads.
    if email.is_empty() || email.chars().any(char::is_whitespace) {
        return Err(bad_request("ユーザー名を入力してください。空白は使えません。").into());
    }

    // Argon2's whole point is to be slow, so what we can cheaply refuse here is
    // a password that breaks the installation's rule.
    crate::app_settings::password_rule(cx)
        .await
        .check(&form.password)
        .map_err(bad_request)?;

    // The first account is the administrator; after that, an invitation is the
    // only way in. Without this the register form would be an open door to
    // anyone who finds the URL.
    // The only self-service registration there is: the first account, which
    // becomes the administrator. After that, accounts are made by that person.
    if !users::none_yet(cx).await? {
        return Err(bad_request("アカウントは管理者が作ります。管理者に頼んでください。").into());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = hash_password(form.password).await?;

    // Nobody is nameless: an account with the field left blank is called by the
    // part of its address in front of the @.
    let display_name = form
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| email.split('@').next().unwrap_or("").to_owned());

    let inserted = sqlx::query(
        "INSERT INTO users (id, email, password_hash, created_at, base_role, display_name)
         VALUES (?1, ?2, ?3, ?4, 'admin', ?5)
         ON CONFLICT (email) DO NOTHING",
    )
    .bind(&id)
    .bind(&email)
    .bind(&password_hash)
    .bind(db::now())
    .bind(&display_name)
    .execute(db::pool(cx))
    .await?;

    if inserted.rows_affected() == 0 {
        return Err(bad_request("そのユーザー名はすでに登録されています。").into());
    }

    start_session(cx, &id).await?;

    Ok(see_other("/"))
}

#[route(POST "/login")]
async fn login(cx: &Cx, Form(form): Form<Credentials>) -> Result<SeeOther> {
    let email = form.email.trim().to_lowercase();
    let keys = ratelimit::keys(cx, &email);

    // Argon2 makes each guess expensive, which is also what a flood of them
    // would exhaust the machine with. Refusing early stops both.
    // 429 would be the right status, but topcoat 0.5.0 has no constructor for
    // it and an unknown error type degrades to 500. A 400 carrying the wait is
    // the honest thing this version can say.
    if let Some(wait) = ratelimit::attempts(cx).retry_after(&keys) {
        let minutes = wait.as_secs().div_ceil(60).max(1);

        return Err(bad_request(format!(
            "ログインの試行が多すぎます。{minutes} 分ほど待ってからお試しください。"
        ))
        .into());
    }

    let found = sqlx::query_as::<_, (String, String)>(
        "SELECT id, password_hash FROM users WHERE email = ?1",
    )
    .bind(&email)
    .fetch_optional(db::pool(cx))
    .await?;

    // Answer an unknown email and a wrong password identically, so the form
    // does not become a way to enumerate who has an account.
    let Some((id, password_hash)) = found else {
        ratelimit::attempts(cx).record_failure(&keys);
        return Err(bad_request("ユーザー名かパスワードが違います。").into());
    };

    if !verify_password(form.password, password_hash).await? {
        ratelimit::attempts(cx).record_failure(&keys);
        return Err(bad_request("ユーザー名かパスワードが違います。").into());
    }

    ratelimit::attempts(cx).forget(&keys);
    start_session(cx, &id).await?;

    Ok(see_other("/"))
}

#[route(POST "/logout")]
async fn logout(cx: &Cx) -> Result<SeeOther> {
    if let Some(hash) = session::stop(cx).await? {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?1")
            .bind(&hash[..])
            .execute(db::pool(cx))
            .await?;
    }

    Ok(see_other("/login"))
}

/// Issues a fresh token and records the session against `user_id`.
async fn start_session(cx: &Cx, user_id: &str) -> Result<()> {
    let session = session::start(cx).await?;

    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES (?1, ?2, ?3)")
        .bind(&session.token_hash[..])
        .bind(user_id)
        .bind(db::unix_seconds(session.expires_at))
        .execute(db::pool(cx))
        .await?;

    Ok(())
}

/// Hashes off the async runtime: Argon2 is deliberately CPU-heavy and would
/// otherwise stall every other request sharing the worker thread.
async fn hash_password(password: String) -> Result<String> {
    let hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);

        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            // `password_hash::Error` is not a `std::error::Error`, so it cannot
            // ride the `?` conversion; its message is all we need.
            .map_err(|error| std::io::Error::other(error.to_string()))
    })
    .await??;

    Ok(hash)
}

/// Whether `password` is this account's current one.
pub async fn password_matches(cx: &Cx, id: &str, password: String) -> Result<bool> {
    let stored = sqlx::query_as::<_, (String,)>("SELECT password_hash FROM users WHERE id = ?1")
        .bind(id)
        .fetch_optional(db::pool(cx))
        .await?;

    let Some((hash,)) = stored else {
        return Ok(false);
    };

    verify_password(password, hash).await
}

async fn verify_password(password: String, password_hash: String) -> Result<bool> {
    let matches = tokio::task::spawn_blocking(move || {
        let Ok(parsed) = PasswordHash::new(&password_hash) else {
            return false;
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
    .await?;

    Ok(matches)
}

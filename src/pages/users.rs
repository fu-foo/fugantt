use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Response,
        content::Form,
        error::{RouterErrorExt, SeeOther, bad_request, see_other},
        page, route,
    },
    view::view,
};

use crate::{auth::require_user, db, users};

/// The people who can sign in. Only an administrator sees this.
#[page("/users")]
async fn index(cx: &Cx) -> Result {
    let user = require_user(cx).await?;
    user.is_admin().then_some(()).ok_or_not_found()?;

    let l = crate::i18n::lang(cx).await;
    let accounts = users::list(cx).await?;

    // The rule lives in the installation settings. Building the description from
    // it too is what keeps the two from drifting apart on the day it changes.
    let rule = crate::app_settings::password_rule(cx).await.describe(l);

    view! {
        <div class="mx-auto w-full max-w-3xl">
            <h1 class="text-2xl font-bold tracking-tight">(l.t("ユーザー"))</h1>
            <p class="mt-1 text-sm text-slate-500">
                (l.t("ベース権限は、プロジェクトに個別の指定が無いときの既定です。「無効」の場合は招待されたプロジェクトのみ表示します。"))
            </p>

            <section class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">(l.t("追加"))</h2>

                <form method="POST" action="/users" class="mt-4 flex flex-wrap items-end gap-3">
                    field(name: "name", label: l.t("名前"), kind: "text", hint: l.t("山田 太郎"))
                    field(name: "email", label: l.t("ユーザー名"), kind: "text", hint: "yamada")
                    field(name: "password", label: l.t("パスワード"), kind: "password", hint: &rule)

                    <div class="flex flex-col gap-1">
                        <label for="base-role" class="text-xs font-medium text-slate-500">
                            (l.t("ベース権限"))
                        </label>
                        <select
                            id="base-role"
                            name="base_role"
                            class="rounded-lg border border-slate-300 px-3 py-2"
                        >
                            for (value, label) in users::ROLES {
                                <option value=(value) selected=((value == "none").then_some("selected"))>
                                    (label)
                                </option>
                            }
                        </select>
                    </div>

                    <button
                        class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                    >
                        (l.t("追加"))
                    </button>
                </form>
            </section>

            <ul class="mt-6 flex flex-col gap-3">
                for account in &accounts {
                    <li class="rounded-xl border border-slate-200 bg-white p-4">
                        <form
                            method="POST"
                            action="/users/update"
                            class="flex flex-wrap items-end gap-3"
                        >
                            <input type="hidden" name="id" value=(&account.id)>

                            <div class="flex flex-col gap-1">
                                <label class="text-xs font-medium text-slate-500">(l.t("名前"))</label>
                                <input
                                    name="name"
                                    value=(&account.display_name)
                                    class="w-40 rounded-lg border border-slate-300 px-3 py-2"
                                >
                            </div>

                            <div class="flex flex-col gap-1">
                                <label class="text-xs font-medium text-slate-500">(l.t("ユーザー名"))</label>
                                <input
                                    name="email"
                                    value=(&account.email)
                                    class="w-40 rounded-lg border border-slate-300 px-3 py-2"
                                >
                            </div>

                            <div class="flex flex-col gap-1">
                                <label class="text-xs font-medium text-slate-500">
                                    (l.t("新しいパスワード（変えるときだけ）"))
                                </label>
                                <input
                                    name="password"
                                    type="password"
                                    placeholder=(l.t("そのまま"))
                                    class="w-40 rounded-lg border border-slate-300 px-3 py-2"
                                >
                            </div>

                            <div class="flex flex-col gap-1">
                                <label class="text-xs font-medium text-slate-500">(l.t("ベース権限"))</label>
                                <select
                                    name="base_role"
                                    class="rounded-lg border border-slate-300 px-3 py-2"
                                >
                                    for (value, label) in users::ROLES {
                                        <option
                                            value=(value)
                                            selected=((value == account.base_role).then_some("selected"))
                                        >
                                            (label)
                                        </option>
                                    }
                                </select>
                            </div>

                            <button
                                class="rounded-lg border border-slate-300 px-4 py-2 hover:bg-slate-50"
                            >
                                (l.t("保存"))
                            </button>
                        </form>

                        if account.id != user.id {
                            <form method="POST" action="/users/remove" class="mt-2">
                                <input type="hidden" name="id" value=(&account.id)>
                                <button
                                    class="text-xs text-slate-400 hover:text-red-600"
                                    onclick=(&format!("return confirm('{}')", l.t("このユーザーを削除します。よろしいですか？")))
                                >
                                    (l.t("削除"))
                                </button>
                            </form>
                        }
                    </li>
                }
            </ul>
        </div>
    }
}

#[topcoat::view::component]
async fn field(name: &str, label: &str, kind: &str, hint: &str) -> Result {
    view! {
        <div class="flex flex-col gap-1">
            <label class="text-xs font-medium text-slate-500">(label)</label>
            <input
                name=(name)
                type=(kind)
                required=""
                placeholder=(hint)
                class="w-40 rounded-lg border border-slate-300 px-3 py-2"
            >
        </div>
    }
}

#[derive(Deserialize)]
struct NewUser {
    name: String,
    email: String,
    password: String,
    base_role: String,
}

#[route(POST "/users")]
async fn create(cx: &Cx, Form(form): Form<NewUser>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    user.is_admin().then_some(()).ok_or_not_found()?;

    let email = form.email.trim().to_lowercase();
    check_login(cx, &email, &form.password).await?;

    let name = form.name.trim();
    let name = if name.is_empty() {
        email.split('@').next().unwrap_or("").to_owned()
    } else {
        name.to_owned()
    };

    if users::name_is_taken(cx, &name, "").await? {
        return Err(bad_request(l.t("その名前は別の人が使っています。担当者は名前で見分けるので、重ならない名前にしてください。")).into());
    }

    let inserted = sqlx::query(
        "INSERT INTO users (id, email, password_hash, created_at, base_role, display_name)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (email) DO NOTHING",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&email)
    .bind(users::hash_password(form.password).await?)
    .bind(db::now())
    .bind(role(&form.base_role))
    .bind(&name)
    .execute(db::pool(cx))
    .await?;

    if inserted.rows_affected() == 0 {
        return Err(bad_request(l.t("そのユーザー名はすでに使われています。")).into());
    }

    Ok(see_other("/users"))
}

#[derive(Deserialize)]
struct EditUser {
    id: String,
    name: String,
    email: String,
    password: String,
    base_role: String,
}

#[route(POST "/users/update")]
async fn update(cx: &Cx, Form(form): Form<EditUser>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    user.is_admin().then_some(()).ok_or_not_found()?;

    let account = users::by_id(cx, &form.id).await?.ok_or_not_found()?;
    let email = form.email.trim().to_lowercase();

    if email.is_empty() || email.chars().any(char::is_whitespace) {
        return Err(bad_request(l.t("ユーザー名を入力してください。空白は使えません。")).into());
    }

    if users::name_is_taken(cx, form.name.trim(), &form.id).await? {
        return Err(bad_request(l.t("その名前は別の人が使っています。担当者は名前で見分けるので、重ならない名前にしてください。")).into());
    }

    // Somebody has to be able to get back in here.
    if account.base_role == "admin" && role(&form.base_role) != "admin" && only_admin(cx).await? {
        return Err(bad_request(l.t("管理者は1人以上必要です。")).into());
    }

    sqlx::query("UPDATE users SET email = ?2, display_name = ?3, base_role = ?4 WHERE id = ?1")
        .bind(&form.id)
        .bind(&email)
        .bind(form.name.trim())
        .bind(role(&form.base_role))
        .execute(db::pool(cx))
        .await?;

    // An assignee is a name, so a rename carries the assignments with it.
    users::rename_everywhere(cx, account.name(), form.name.trim()).await?;

    if !form.password.is_empty() {
        set_password(cx, &form.id, form.password).await?;
    }

    Ok(see_other("/users"))
}

#[derive(Deserialize)]
struct RemoveUser {
    id: String,
}

#[route(POST "/users/remove")]
async fn remove(cx: &Cx, Form(form): Form<RemoveUser>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    user.is_admin().then_some(()).ok_or_not_found()?;

    if form.id == user.id {
        return Err(bad_request(l.t("自分は削除できません。")).into());
    }

    sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(&form.id)
        .execute(db::pool(cx))
        .await?;

    Ok(see_other("/users"))
}

/// Your own settings: the two things nobody should have to ask for.
#[page("/me")]
async fn me(cx: &Cx) -> Result {
    let user = require_user(cx).await?;

    let l = crate::i18n::lang(cx).await;
    let rule = crate::app_settings::password_rule(cx).await.describe(l);

    view! {
        <div class="mx-auto w-full max-w-xl">
            <h1 class="text-2xl font-bold tracking-tight">(l.t("自分の設定"))</h1>
            <p class="mt-1 text-sm text-slate-500">
                (&l.about("ユーザー名（{}）とベース権限は管理者が決めます。", &user.email))
            </p>

            <section class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">(l.t("名前"))</h2>
                <p class="mt-1 text-xs text-slate-500">
                    (l.t("画面に出る名前です。変えると、担当者に入っている自分の名前も一緒に変わります。"))
                </p>

                <form method="POST" action="/me/name" class="mt-4 flex flex-wrap items-end gap-3">
                    <input
                        name="name"
                        value=(&user.display_name)
                        required=""
                        class="w-56 rounded-lg border border-slate-300 px-3 py-2"
                    >

                    <div class="flex flex-col gap-1">
                        <label for="language" class="text-xs font-medium text-slate-500">(l.t("言語"))</label>
                        <select
                            id="language"
                            name="language"
                            class="rounded-lg border border-slate-300 px-3 py-2"
                        >
                            for (value, label) in [
                                ("", l.t("全体の設定に従う")),
                                ("ja", l.t("日本語")),
                                ("en", "English"),
                            ] {
                                <option
                                    value=(value)
                                    selected=((user.language == value).then_some("selected"))
                                >
                                    (label)
                                </option>
                            }
                        </select>
                    </div>

                    <button
                        class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                    >
                        (l.t("保存"))
                    </button>
                </form>
            </section>

            <section class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">(l.t("外観"))</h2>
                <p class="mt-1 text-xs text-slate-500">
                    (l.t("自分の画面にだけ効きます。ほかの人には見えません。"))
                </p>

                <form method="POST" action="/me/look" class="mt-4 flex flex-col gap-4">
                    <div class="flex flex-col gap-1">
                        <label for="theme" class="text-xs font-medium text-slate-500">
                            (l.t("テーマ"))
                        </label>
                        <select
                            id="theme"
                            name="theme"
                            class="w-56 rounded-lg border border-slate-300 px-3 py-2"
                        >
                            for (value, label) in [
                                ("", l.t("自動（OSに合わせる）")),
                                ("light", l.t("明るい")),
                                ("dark", l.t("暗い")),
                            ] {
                                <option
                                    value=(value)
                                    selected=((user.theme == value).then_some("selected"))
                                >
                                    (l.t(label))
                                </option>
                            }
                        </select>
                    </div>

                    <div class="flex flex-col gap-1">
                        <label for="css" class="text-xs font-medium text-slate-500">
                            (l.t("自分用の CSS"))
                        </label>
                        <textarea
                            id="css"
                            name="css"
                            rows="8"
                            spellcheck="false"
                            placeholder=(".fg-bar { border-radius: 0 }")
                            class="w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-xs"
                        >(&user.custom_css)</textarea>
                        <p class="text-xs text-slate-400">
                            (l.t("最後に読み込まれるため、ここでの指定が優先されます。2万文字まで。@import は使えません。"))
                        </p>
                    </div>

                    <button
                        class="w-fit rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                    >
                        (l.t("保存"))
                    </button>
                </form>
            </section>

            <section class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">(l.t("パスワード"))</h2>

                <form method="POST" action="/me/password" class="mt-4 flex flex-col gap-3">
                    <label class="text-xs font-medium text-slate-500">(l.t("いまのパスワード"))</label>
                    <input
                        name="current"
                        type="password"
                        required=""
                        class="rounded-lg border border-slate-300 px-3 py-2"
                    >

                    <label class="text-xs font-medium text-slate-500">(l.t("新しいパスワード"))</label>
                    <input
                        name="password"
                        type="password"
                        required=""
                        class="rounded-lg border border-slate-300 px-3 py-2"
                    >
                    <p class="text-xs text-slate-500">(&rule)</p>

                    <button
                        class="w-fit rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                    >
                        (l.t("変更"))
                    </button>
                </form>
            </section>
        </div>
    }
}

#[derive(Deserialize)]
struct MyName {
    name: String,
    /// Empty means undecided, which follows the installation setting.
    language: Option<String>,
}

#[route(POST "/me/name")]
async fn set_my_name(cx: &Cx, Form(form): Form<MyName>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;
    let name = form.name.trim();

    if name.is_empty() {
        return Err(bad_request(l.t("名前を入力してください。")).into());
    }

    if users::name_is_taken(cx, name, &user.id).await? {
        return Err(
            bad_request(l.t("その名前は別の人が使っています。別の名前にしてください。")).into(),
        );
    }

    let language = match form.language.as_deref().unwrap_or("").trim() {
        // A value we cannot read becomes "undecided". Falling back to the
        // installation setting beats a broken value quietly changing the language.
        value @ ("ja" | "en") => value,
        _ => "",
    };

    sqlx::query("UPDATE users SET display_name = ?2, language = ?3 WHERE id = ?1")
        .bind(&user.id)
        .bind(name)
        .bind(language)
        .execute(db::pool(cx))
        .await?;

    users::rename_everywhere(cx, user.display(), name).await?;

    Ok(see_other("/me"))
}

/// How the screen looks, for one person.
#[derive(Deserialize)]
struct MyLook {
    theme: String,
    css: String,
}

/// Free text that ends up in a stylesheet, kept to what a stylesheet can hold.
///
/// Angle brackets go, because this is served as its own file and one day
/// somebody will paste it into a page instead. `@import` is defanged because it
/// fetches from wherever it is pointed, and a settings box is not the place to
/// arrange that. Neither is a security boundary — the sheet only ever reaches the
/// person who wrote it — they are the two ways a stylesheet stops being one.
fn tidy_css(raw: &str) -> String {
    raw.replace(['<', '>'], "")
        // Not wrapped in a comment: this runs again when the sheet is served,
        // and a comment inside a comment ends at the first `*/`.
        .replace("@import", "x-import")
        .chars()
        .take(20_000)
        .collect()
}

#[route(POST "/me/look")]
async fn set_my_look(cx: &Cx, Form(form): Form<MyLook>) -> Result<SeeOther> {
    let user = require_user(cx).await?;

    let theme = match form.theme.trim() {
        value @ ("light" | "dark") => value,
        // Anything else means "ask the machine", which is also what a value
        // this build does not understand should do.
        _ => "",
    };

    sqlx::query("UPDATE users SET theme = ?2, custom_css = ?3 WHERE id = ?1")
        .bind(&user.id)
        .bind(theme)
        .bind(tidy_css(&form.css))
        .execute(db::pool(cx))
        .await?;

    Ok(see_other("/me"))
}

/// One person's own stylesheet, served to that person.
///
/// A route rather than a `<style>` block: it is cached like any other
/// stylesheet, it cannot end early on a stray `</style>`, and the browser
/// reports its mistakes as CSS rather than as a broken page.
#[route(GET "/me/custom.css")]
async fn my_css(cx: &Cx) -> Result<Response> {
    let user = require_user(cx).await?;

    Ok(Response::builder()
        .header("Content-Type", "text/css; charset=utf-8")
        // Nobody else's, so nobody else's cache either.
        .header("Cache-Control", "private, no-cache")
        .body(Body::from(tidy_css(&user.custom_css)))?)
}

#[derive(Deserialize)]
struct MyPassword {
    current: String,
    password: String,
}

#[route(POST "/me/password")]
async fn set_my_password(cx: &Cx, Form(form): Form<MyPassword>) -> Result<SeeOther> {
    let l = crate::i18n::lang(cx).await;
    let user = require_user(cx).await?;

    // The current one, because a session left open on somebody's desk is not a
    // reason to let the next person past change the password.
    if !crate::auth::password_matches(cx, &user.id, form.current).await? {
        return Err(bad_request(l.t("いまのパスワードが違います。")).into());
    }

    set_password(cx, &user.id, form.password).await?;

    Ok(see_other("/me"))
}

async fn set_password(cx: &Cx, id: &str, password: String) -> Result<()> {
    // One rule, in the installation settings. Creating an account, changing your
    // own password, and an administrator resetting one all pass through it.
    crate::app_settings::password_rule(cx)
        .await
        .check(&password)
        .map_err(bad_request)?;

    sqlx::query("UPDATE users SET password_hash = ?2 WHERE id = ?1")
        .bind(id)
        .bind(users::hash_password(password).await?)
        .execute(db::pool(cx))
        .await?;

    // Half the reasons to change a password are "it may have leaked", so every
    // other session is closed with it. The one doing the changing stays open —
    // there is no sense in throwing somebody back to the sign-in page for it. An
    // administrator resetting someone else's password closes all of theirs.
    match topcoat::session::token_hash(cx).await? {
        Some(current) => {
            sqlx::query("DELETE FROM sessions WHERE user_id = ?1 AND token_hash <> ?2")
                .bind(id)
                .bind(&current[..])
                .execute(db::pool(cx))
                .await?
        }
        None => {
            sqlx::query("DELETE FROM sessions WHERE user_id = ?1")
                .bind(id)
                .execute(db::pool(cx))
                .await?
        }
    };

    Ok(())
}

async fn check_login(cx: &Cx, email: &str, password: &str) -> Result<()> {
    let l = crate::i18n::lang(cx).await;
    if email.is_empty() || email.chars().any(char::is_whitespace) {
        return Err(bad_request(l.t("ユーザー名を入力してください。空白は使えません。")).into());
    }

    crate::app_settings::password_rule(cx)
        .await
        .check(password)
        .map_err(bad_request)?;

    Ok(())
}

/// The stored value for whatever the form said.
fn role(value: &str) -> &'static str {
    users::ROLES
        .iter()
        .find(|(key, _)| *key == value)
        .map_or("none", |(key, _)| *key)
}

async fn only_admin(cx: &Cx) -> Result<bool> {
    let (admins,) =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM users WHERE base_role = 'admin'")
            .fetch_one(db::pool(cx))
            .await?;

    Ok(admins <= 1)
}

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{RouterErrorExt, SeeOther, bad_request, see_other},
        page, route,
    },
    view::view,
};

use crate::{app_settings, auth::require_user, db, project};

/// Settings that belong to the installation. Only an administrator sees this.
#[page("/admin")]
async fn index(cx: &Cx) -> Result {
    let user = require_user(cx).await?;

    // Not-found rather than forbidden: there is no reason to tell a normal
    // member that an administration page exists.
    user.is_admin().then_some(()).ok_or_not_found()?;

    let l = crate::i18n::lang(cx).await;
    let name = app_settings::name(cx).await;
    let eras = app_settings::eras_text(cx).await;
    let language = app_settings::get(cx, "language")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "auto".to_owned());
    let rule = app_settings::password_rule(cx).await;
    let banned = app_settings::banned_text(cx).await;
    let holidays = project::app_holidays(cx).await?;
    let assignees = project::assignee_master(cx).await?;

    view! {
        <div class="mx-auto w-full max-w-3xl">
            <h1 class="text-2xl font-bold tracking-tight">"全体の設定"</h1>
            <p class="mt-1 text-sm text-slate-500">"この fugantt を使う全員に効きます。"</p>

            <form
                method="POST"
                action="/admin"
                class="mt-6 flex flex-col gap-6 rounded-xl border border-slate-200 bg-white p-6"
            >
                <div class="flex flex-col gap-1">
                    <label for="app-name" class="text-xs font-medium text-slate-500">
                        (l.t("アプリの名前（左上に出ます）"))
                    </label>
                    <input
                        id="app-name"
                        name="app_name"
                        value=(&name)
                        placeholder=(app_settings::DEFAULT_NAME)
                        class="w-64 rounded-lg border border-slate-300 px-3 py-2"
                    >
                </div>

                <div class="flex flex-col gap-1">
                    <label for="language" class="text-xs font-medium text-slate-500">"言語"</label>
                    <select
                        id="language"
                        name="language"
                        class="w-64 rounded-lg border border-slate-300 px-3 py-2"
                    >
                        for (value, label) in [
                            ("auto", "自動（ブラウザに合わせる）"),
                            ("ja", "日本語"),
                            ("en", "English"),
                        ] {
                            <option
                                value=(value)
                                selected=((language == value).then_some("selected"))
                            >
                                (label)
                            </option>
                        }
                    </select>
                    <p class="text-xs text-slate-500">
                        (l.t("既定です。自分の設定で選んだ人は、そちらが優先されます。「自動」は、その人のブラウザ（OS）の言語に合わせます。"))
                    </p>
                </div>

                <div class="flex flex-col gap-1">
                    <label for="eras" class="text-xs font-medium text-slate-500">"元号"</label>
                    <textarea
                        id="eras"
                        name="eras"
                        rows="5"
                        class="w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-sm"
                    >(&eras)</textarea>
                    <p class="text-xs text-slate-500">
                        (l.t("1行に「開始日 名称」。新しい元号が決まったら、ここに1行足すだけで済みます。読めない行は無視します。"))
                    </p>
                </div>

                <button
                    class="self-start rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                >
                    (l.t("保存"))
                </button>
            </form>

            // --- passwords --------------------------------------------------

            <section id="password" class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">"パスワードの決まり"</h2>
                <p class="mt-1 text-sm text-slate-500">
                    (l.t("新しく決めるときだけ効きます。いま使っているパスワードは、次に変えるまでそのまま使えます。"))
                </p>

                <form method="POST" action="/admin/password" class="mt-4 flex flex-col gap-6">
                    <div class="flex flex-wrap items-end gap-6">
                        <div class="flex flex-col gap-1">
                            <label for="password-min" class="text-xs font-medium text-slate-500">
                                (l.t("最低文字数"))
                            </label>
                            <input
                                id="password-min"
                                name="password_min"
                                type="number"
                                min="4"
                                max="128"
                                value=(rule.min.to_string())
                                class="w-28 rounded-lg border border-slate-300 px-3 py-2"
                            >
                            <span class="text-xs text-slate-400">"バイトではなく文字で数えます"</span>
                        </div>

                        <div class="flex flex-col gap-1">
                            <span class="text-xs font-medium text-slate-500">"必ず入れる文字"</span>
                            <div class="flex flex-wrap items-center gap-4 py-2">
                                for kind in app_settings::Kind::ALL {
                                    <label class="flex items-center gap-1.5 text-sm">
                                        <input
                                            type="checkbox"
                                            name=(&format!("kind_{}", kind.key()))
                                            checked=(rule.kinds.contains(&kind).then_some("checked"))
                                            class="size-4 rounded border-slate-300"
                                        >
                                        (kind.label())
                                    </label>
                                }
                            </div>
                            <span class="text-xs text-slate-400">
                                (l.t("何もチェックしなければ指定なし。日本語は記号に数えます"))
                            </span>
                        </div>
                    </div>

                    <div class="flex flex-col gap-1">
                        <label for="password-banned" class="text-xs font-medium text-slate-500">
                            (l.t("使わせない語"))
                        </label>
                        <textarea
                            id="password-banned"
                            name="password_banned"
                            rows="6"
                            class="w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-sm"
                        >(&banned)</textarea>
                        <p class="text-xs text-slate-500">
                            (l.t("1行に1語。これを含むパスワードは断ります（大文字小文字は問いません）。会社名や製品名を足しておくと効きます。空にすれば、この検査はしません。"))
                        </p>
                    </div>

                    <p class="text-sm text-slate-600">
                        "いまの決まり: "<span class="font-medium">(&rule.describe())</span>
                    </p>

                    <button
                        class="self-start rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                    >
                        (l.t("保存"))
                    </button>
                </form>
            </section>

            // --- holidays ---------------------------------------------------

            <section id="holidays" class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">
                    (l.t("祝日・休業日"))
                    <span class="ml-2 text-sm font-normal text-slate-400">
                        (&format!("{} 日", holidays.len()))
                    </span>
                </h2>
                <p class="mt-1 text-sm text-slate-500">
                    (l.t("会社の暦です。すべてのプロジェクトが、まずここを見ます。現場ごとの違いは、それぞれのプロジェクトの設定で足したり外したりできます。"))
                </p>

                <form
                    method="POST"
                    action="/admin/holidays/japan"
                    class="mt-4 flex flex-wrap items-end gap-3 rounded-lg bg-slate-50 p-3"
                >
                    <div class="flex flex-col gap-1">
                        <label for="holiday-year" class="text-xs font-medium text-slate-500">
                            (l.t("日本の祝日をまとめて入れる"))
                        </label>
                        <input
                            id="holiday-year"
                            name="year"
                            type="number"
                            min="2020"
                            max="2099"
                            value=(jiff::Zoned::now().year())
                            class="w-28 rounded-lg border border-slate-300 px-3 py-2"
                        >
                    </div>
                    <button class="rounded-lg border border-slate-300 bg-white px-4 py-2 hover:bg-slate-100">
                        (l.t("取り込む"))
                    </button>
                    <span class="text-xs text-slate-500">
                        (l.t("振替休日と国民の休日も計算します。同じ日付が既にあれば残します。"))
                    </span>
                </form>

                <form method="POST" action="/admin/holidays" class="mt-4 flex flex-wrap items-end gap-3">
                    <div class="flex flex-col gap-1">
                        <label for="holiday-date" class="text-xs font-medium text-slate-500">"日付"</label>
                        <input
                            id="holiday-date"
                            name="date"
                            type="date"
                            required=""
                            class="rounded-lg border border-slate-300 px-3 py-2"
                        >
                    </div>

                    <div class="flex flex-1 flex-col gap-1">
                        <label for="holiday-name" class="text-xs font-medium text-slate-500">"名称"</label>
                        <input
                            id="holiday-name"
                            name="name"
                            placeholder=(l.t("創立記念日"))
                            class="w-full rounded-lg border border-slate-300 px-3 py-2"
                        >
                    </div>

                    <button class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500">
                        (l.t("追加"))
                    </button>
                </form>

                if holidays.is_empty() {
                    <p class="mt-6 text-sm text-slate-400">"まだ登録がありません。"</p>
                } else {
                    <ul class="mt-6 divide-y divide-slate-100 border-t border-slate-100">
                        for holiday in &holidays {
                            <li class="flex items-center gap-4 py-2.5">
                                <span class="w-28 font-mono text-sm tabular-nums">(&holiday.date)</span>
                                <span class="text-sm text-slate-600">(&holiday.name)</span>

                                <form method="POST" action="/admin/holidays/remove" class="ml-auto">
                                    <input type="hidden" name="date" value=(&holiday.date)>
                                    <button class="text-sm text-slate-400 hover:text-red-600">"削除"</button>
                                </form>
                            </li>
                        }
                    </ul>
                }
            </section>

            // --- assignee colours -------------------------------------------

            <section id="assignees" class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">
                    (l.t("担当者の色"))
                    <span class="ml-2 text-sm font-normal text-slate-400">
                        (&format!("{} 人", assignees.len()))
                    </span>
                </h2>
                <p class="mt-1 text-sm text-slate-500">
                    (l.t("同じ人はどのプロジェクトでも同じ色にします。アカウントの無い名前（協力会社・他部署）も登録できます。"))
                </p>

                <form method="POST" action="/admin/assignees" class="mt-4 flex flex-wrap items-end gap-3">
                    <div class="flex flex-col gap-1">
                        <label for="assignee-name" class="text-xs font-medium text-slate-500">"名前"</label>
                        <input
                            id="assignee-name"
                            name="name"
                            required=""
                            placeholder=(l.t("協力会社 A"))
                            class="rounded-lg border border-slate-300 px-3 py-2"
                        >
                    </div>

                    <div class="flex flex-col gap-1">
                        <label for="assignee-color" class="text-xs font-medium text-slate-500">"文字色"</label>
                        <input
                            id="assignee-color"
                            name="color"
                            type="color"
                            value="#1e3a8a"
                            class="h-10 w-16 rounded-lg border border-slate-300"
                        >
                    </div>

                    <div class="flex flex-col gap-1">
                        <label for="assignee-background" class="text-xs font-medium text-slate-500">
                            (l.t("背景色"))
                        </label>
                        <input
                            id="assignee-background"
                            name="background"
                            type="color"
                            value="#dbeafe"
                            class="h-10 w-16 rounded-lg border border-slate-300"
                        >
                    </div>

                    <button class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500">
                        (l.t("保存"))
                    </button>
                </form>

                if assignees.is_empty() {
                    <p class="mt-6 text-sm text-slate-400">
                        (l.t("まだ登録がありません。プロジェクトで使われている名前は、そのまま色なしで出ます。"))
                    </p>
                } else {
                    <ul class="mt-6 divide-y divide-slate-100 border-t border-slate-100">
                        for person in &assignees {
                            <li class="flex items-center gap-4 py-2.5">
                                <span
                                    class="rounded-full px-2.5 py-0.5 text-sm"
                                    style=(&format!(
                                        "color: {}; background: {}",
                                        if person.color.is_empty() { "#334155" } else { &person.color },
                                        if person.background.is_empty() { "#f1f5f9" } else { &person.background },
                                    ))
                                >
                                    (&person.name)
                                </span>

                                <form
                                    method="POST"
                                    action="/admin/assignees"
                                    class="ml-auto flex items-center gap-2"
                                >
                                    <input type="hidden" name="name" value=(&person.name)>
                                    <input
                                        name="color"
                                        type="color"
                                        value=(if person.color.is_empty() { "#334155" } else { &person.color })
                                        class="h-8 w-12 rounded border border-slate-300"
                                    >
                                    <input
                                        name="background"
                                        type="color"
                                        value=(
                                            if person.background.is_empty() { "#f1f5f9" }
                                            else { &person.background }
                                        )
                                        class="h-8 w-12 rounded border border-slate-300"
                                    >
                                    <button class="rounded-lg border border-slate-300 px-3 py-1 text-sm hover:bg-slate-100">
                                        (l.t("更新"))
                                    </button>
                                </form>

                                <form method="POST" action="/admin/assignees/remove">
                                    <input type="hidden" name="name" value=(&person.name)>
                                    <button class="text-sm text-slate-400 hover:text-red-600">"色を外す"</button>
                                </form>
                            </li>
                        }
                    </ul>
                }
            </section>
        </div>
    }
}

#[derive(Deserialize)]
struct AppForm {
    app_name: String,
    eras: String,
    language: String,
}

#[route(POST "/admin")]
async fn save(cx: &Cx, Form(form): Form<AppForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;

    user.is_admin().then_some(()).ok_or_not_found()?;

    app_settings::set(cx, "app_name", form.app_name.trim()).await?;
    app_settings::set(cx, "eras", form.eras.trim()).await?;
    app_settings::set(
        cx,
        "language",
        match form.language.trim() {
            value @ ("ja" | "en") => value,
            _ => "auto",
        },
    )
    .await?;

    Ok(see_other("/admin"))
}

#[derive(Deserialize)]
struct PasswordForm {
    password_min: String,
    password_banned: String,
    // An unticked checkbox posts nothing at all.
    kind_lower: Option<String>,
    kind_upper: Option<String>,
    kind_digit: Option<String>,
    kind_symbol: Option<String>,
}

#[route(POST "/admin/password")]
async fn save_password_rule(cx: &Cx, Form(form): Form<PasswordForm>) -> Result<SeeOther> {
    require_admin(cx).await?;

    let min: usize = form
        .password_min
        .trim()
        .parse()
        .map_err(|_| bad_request("最低文字数は数字で入れてください。"))?;

    if !(4..=128).contains(&min) {
        return Err(bad_request("最低文字数は4〜128の範囲で決めてください。").into());
    }

    let kinds: Vec<&str> = [
        (form.kind_lower.is_some(), app_settings::Kind::Lower),
        (form.kind_upper.is_some(), app_settings::Kind::Upper),
        (form.kind_digit.is_some(), app_settings::Kind::Digit),
        (form.kind_symbol.is_some(), app_settings::Kind::Symbol),
    ]
    .into_iter()
    .filter_map(|(ticked, kind)| ticked.then_some(kind.key()))
    .collect();

    app_settings::set(cx, "password_min", &min.to_string()).await?;
    app_settings::set(cx, "password_kinds", &kinds.join(",")).await?;
    // Saved empty means no check at all, and the emptiness is remembered so it
    // does not silently return to the default.
    app_settings::set(cx, "password_banned", form.password_banned.trim()).await?;

    Ok(see_other("/admin#password"))
}

/// Everything below changes what every project sees, so every project's
/// revision moves with it — an open screen has no other way to notice.
async fn require_admin(cx: &Cx) -> Result<()> {
    let user = require_user(cx).await?;
    user.is_admin().then_some(()).ok_or_not_found()?;
    Ok(())
}

#[derive(Deserialize)]
struct HolidayForm {
    date: String,
    name: Option<String>,
}

#[route(POST "/admin/holidays")]
async fn add_holiday(cx: &Cx, Form(form): Form<HolidayForm>) -> Result<SeeOther> {
    require_admin(cx).await?;

    let date: jiff::civil::Date = form
        .date
        .trim()
        .parse()
        .map_err(|_| bad_request("日付は YYYY-MM-DD の形式で入力してください。"))?;

    sqlx::query(
        "INSERT INTO app_holidays (date, name) VALUES (?1, ?2)
         ON CONFLICT (date) DO UPDATE SET name = excluded.name",
    )
    .bind(date.to_string())
    .bind(form.name.as_deref().unwrap_or("").trim())
    .execute(db::pool(cx))
    .await?;

    project::bump_everything(cx).await?;

    Ok(see_other("/admin#holidays"))
}

#[derive(Deserialize)]
struct ImportHolidays {
    year: i16,
}

/// Fills in a year of Japanese public holidays.
///
/// Existing entries keep their names: someone who renamed 8月11日 to 「夏季
/// 休業」 meant it.
#[route(POST "/admin/holidays/japan")]
async fn import_japanese_holidays(cx: &Cx, Form(form): Form<ImportHolidays>) -> Result<SeeOther> {
    require_admin(cx).await?;

    if !(2020..=2099).contains(&form.year) {
        return Err(bad_request("2020〜2099 年に対応しています。").into());
    }

    let mut tx = db::pool(cx).begin().await?;

    for (date, name) in crate::holidays::japanese(form.year) {
        sqlx::query(
            "INSERT INTO app_holidays (date, name) VALUES (?1, ?2)
             ON CONFLICT (date) DO NOTHING",
        )
        .bind(date.to_string())
        .bind(name)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    project::bump_everything(cx).await?;

    Ok(see_other("/admin#holidays"))
}

#[derive(Deserialize)]
struct RemoveHoliday {
    date: String,
}

#[route(POST "/admin/holidays/remove")]
async fn remove_holiday(cx: &Cx, Form(form): Form<RemoveHoliday>) -> Result<SeeOther> {
    require_admin(cx).await?;

    sqlx::query("DELETE FROM app_holidays WHERE date = ?1")
        .bind(form.date.trim())
        .execute(db::pool(cx))
        .await?;

    project::bump_everything(cx).await?;

    Ok(see_other("/admin#holidays"))
}

#[derive(Deserialize)]
struct AssigneeForm {
    name: String,
    color: Option<String>,
    background: Option<String>,
}

#[route(POST "/admin/assignees")]
async fn set_assignee(cx: &Cx, Form(form): Form<AssigneeForm>) -> Result<SeeOther> {
    require_admin(cx).await?;

    let name = form.name.trim();
    if name.is_empty() {
        return Err(bad_request("名前を入力してください。").into());
    }

    let colour = |value: Option<&str>| -> Result<String> {
        let value = value.unwrap_or("").trim();

        if value.is_empty() {
            return Ok(String::new());
        }
        if !crate::domain::is_hex_colour(value) {
            return Err(bad_request("色は #rrggbb の形式で指定してください。").into());
        }

        Ok(value.to_owned())
    };

    sqlx::query(
        "INSERT INTO assignees (name, color, background) VALUES (?1, ?2, ?3)
         ON CONFLICT (name) DO UPDATE
            SET color = excluded.color, background = excluded.background",
    )
    .bind(name)
    .bind(colour(form.color.as_deref())?)
    .bind(colour(form.background.as_deref())?)
    .execute(db::pool(cx))
    .await?;

    project::bump_everything(cx).await?;

    Ok(see_other("/admin#assignees"))
}

#[derive(Deserialize)]
struct RemoveAssignee {
    name: String,
}

/// Drops the colouring. The name itself lives on the plans that use it, so it
/// keeps showing up there — without a colour.
#[route(POST "/admin/assignees/remove")]
async fn remove_assignee(cx: &Cx, Form(form): Form<RemoveAssignee>) -> Result<SeeOther> {
    require_admin(cx).await?;

    sqlx::query("DELETE FROM assignees WHERE name = ?1")
        .bind(form.name.trim())
        .execute(db::pool(cx))
        .await?;

    project::bump_everything(cx).await?;

    Ok(see_other("/admin#assignees"))
}

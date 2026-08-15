use sqlx::FromRow;
use topcoat::{
    Result,
    context::Cx,
    router::{page, query_params},
    view::component,
    view::view,
};

use crate::{auth::require_user, db, project};

/// Which section the page came back to, after a save inside it.
///
/// A form posts and the page is built again, which closes every `<details>` and
/// throws the scroll away. The redirect carries the section's name so the page
/// can open it, and the same name is the fragment the browser scrolls to.
#[query_params(error = not_found())]
struct OpenSection {
    open: Option<String>,
    /// A token, shown once, on the way back from making one.
    issued: Option<String>,
}

#[derive(FromRow)]
struct Member {
    user_id: String,
    email: String,
    display_name: String,
    role: String,
}

/// A timestamp as a day. When a token was last used matters; the minute does not.
pub(super) fn used_on(at: i64) -> String {
    jiff::Timestamp::from_second(at)
        .map(|stamp| {
            stamp
                .to_zoned(jiff::tz::TimeZone::system())
                .strftime("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_default()
}

/// Project settings: the calendar the chart shades, and who may touch it.
#[page("/projects/{project_id}/settings")]
async fn settings(cx: &Cx) -> Result {
    let user = require_user(cx).await?;
    let project_id = project::id_from_path(cx)?.to_owned();
    let project = project::authorize(cx, &user.id, &project_id).await?;
    let query = query_params::<OpenSection>(cx)?;
    let open = query.open.clone().unwrap_or_default();
    // Shown once and never stored, so leaving this page loses it for good.
    let issued = query.issued.clone().unwrap_or_default();

    let l = crate::i18n::lang(cx).await;
    let tokens = crate::tokens::list(cx, &project.id).await?;

    let usage = format!(
        "curl -H 'Authorization: Bearer fug_…' \\\n  {origin}/api/projects/{id}/document\n\n         curl -X POST -H 'Authorization: Bearer fug_…' \\\n  -H 'Content-Type: application/json' \\\n           --data @plan.json \\\n  {origin}/api/projects/{id}/document",
        origin = "https://…",
        id = project.id,
    );

    // Only people who already have an account can be added to a project.
    let accounts = crate::users::list(cx).await?;

    // What the page shows is the days this project actually takes off: the shared
    // calendar plus and minus this project's own difference. Each row says which
    // of the two it came from.
    let holidays = project::holidays(cx, &project.id).await?;

    let shared: std::collections::HashSet<String> = project::app_holidays(cx)
        .await?
        .into_iter()
        .map(|holiday| holiday.date)
        .collect();

    // Shared holidays this particular project works through.
    let skipped: Vec<String> = project::holiday_diff(cx, &project.id)
        .await?
        .into_iter()
        .filter(|(_, _, kind)| kind == "skip")
        .map(|(date, _, _)| date)
        .collect();

    let data = project::grid_data(cx, &project).await?;
    let theme = &data.theme;

    // The columns: order, visibility and width, all in one place.
    struct Column {
        key: String,
        label: String,
        shown: bool,
        width: String,
    }

    let columns: Vec<Column> = project::column_order(&data)
        .into_iter()
        .map(|key| Column {
            label: data
                .fields
                .iter()
                .find(|field| field.id == key)
                .map(|field| field.label.clone())
                .unwrap_or_else(|| column_label(&key).to_owned()),
            shown: !data.hidden_columns.contains(&key),
            width: data
                .column_widths
                .get(&key)
                .map(ToString::to_string)
                .unwrap_or_default(),
            key,
        })
        .collect();

    // Whether 完了 means 100% depends on the statuses, so the wording is built
    // from them rather than written out here.
    let linked_label = {
        let pairs: Vec<String> = data
            .statuses
            .iter()
            .filter_map(|status| {
                status
                    .percent
                    .map(|percent| format!("{}→{percent}%", status.name))
            })
            .collect();

        if pairs.is_empty() {
            "ステータスに連動（進捗を決めたステータスがまだありません）".to_owned()
        } else {
            format!("ステータスに連動（{}）", pairs.join(" / "))
        }
    };

    let linked = project::settings(cx, &project.id)
        .await?
        .get("progress_mode")
        .is_some_and(|mode| mode == "status");

    let members = sqlx::query_as::<_, Member>(
        "SELECT project_members.user_id, users.email, users.display_name,
                project_members.role
           FROM project_members
           JOIN users ON users.id = project_members.user_id
          WHERE project_members.project_id = ?1
          ORDER BY users.email",
    )
    .bind(&project.id)
    .fetch_all(db::pool(cx))
    .await?;

    view! {
        <div class="mx-auto w-full max-w-3xl">
            <h1 class="text-2xl font-bold tracking-tight">"設定"</h1>

            // --- view -------------------------------------------------------

            <section id="view" class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">"表示"</h2>

                if !project.can_edit() {
                    <p class="mt-3 text-sm text-slate-600">
                        (l.t("日数の数え方は編集者が設定します。"))
                    </p>
                }

                if project.can_edit() {
                <form
                    method="POST"
                    action=(("/projects/", &project.id, "/view"))
                    class="mt-4 flex flex-col gap-4"
                >
                    <fieldset class="flex flex-col gap-2">
                        <legend class="text-xs font-medium text-slate-500">"日数から除く日"</legend>
                        <p class="text-xs text-slate-400">
                            (l.t("週の何曜を休みにするかは現場ごと。既定はどれも外しません。"))
                        </p>

                        count_switch(name: "skip_monday", label: "月曜", on: data.counting.monday)
                        count_switch(name: "skip_tuesday", label: "火曜", on: data.counting.tuesday)
                        count_switch(
                            name: "skip_wednesday",
                            label: "水曜",
                            on: data.counting.wednesday,
                        )
                        count_switch(
                            name: "skip_thursday",
                            label: "木曜",
                            on: data.counting.thursday,
                        )
                        count_switch(name: "skip_friday", label: "金曜", on: data.counting.friday)
                        count_switch(
                            name: "skip_saturday",
                            label: "土曜",
                            on: data.counting.saturday,
                        )
                        count_switch(
                            name: "skip_sunday",
                            label: "日曜",
                            on: data.counting.sunday,
                        )
                        count_switch(
                            name: "skip_holidays",
                            label: "祝日・休業日",
                            on: data.counting.holidays,
                        )
                        count_switch(
                            name: "skip_leave",
                            label: "担当者の休暇",
                            on: data.counting.leave,
                        )
                    </fieldset>

                    <div class="flex flex-col gap-1">
                        <label for="progress-mode" class="text-xs font-medium text-slate-500">
                            (l.t("進捗の入れ方"))
                        </label>
                        <select
                            id="progress-mode"
                            name="progress_mode"
                            class="w-64 rounded-lg border border-slate-300 px-3 py-2"
                        >
                            <option value="" selected=((!linked).then_some("selected"))>
                                (l.t("手入力"))
                            </option>
                            <option value="status" selected=(linked.then_some("selected"))>
                                (&linked_label)
                            </option>
                        </select>
                        <p class="text-xs text-slate-500">
                            (l.t("連動しても、進捗を決めていないステータスは手入力のままです。進捗を 100% にすると、実施終了が空なら今日の日付が入ります。"))
                        </p>
                    </div>

                    <div class="flex flex-wrap items-end gap-4">
                        <div class="flex flex-col gap-1">
                            <label for="fiscal" class="text-xs font-medium text-slate-500">
                                (l.t("年度の開始月"))
                            </label>
                            <select
                                id="fiscal"
                                name="fiscal_year_start"
                                class="rounded-lg border border-slate-300 px-3 py-2"
                            >
                                for month in 1..=12u32 {
                                    <option
                                        value=(month.to_string())
                                        selected=(
                                            (month == data.fiscal_year_start).then_some("selected")
                                        )
                                    >
                                        (month.to_string())
                                        (l.t("月"))
                                    </option>
                                }
                            </select>
                        </div>

                        <div class="flex flex-col gap-1">
                            <label for="frozen" class="text-xs font-medium text-slate-500">
                                (l.t("固定する列"))
                            </label>
                            <select
                                id="frozen"
                                name="frozen_columns"
                                class="rounded-lg border border-slate-300 px-3 py-2"
                            >
                                for count in 0..=6usize {
                                    <option
                                        value=(count.to_string())
                                        selected=((count == data.frozen_columns).then_some("selected"))
                                    >
                                        if count == 0 { "固定しない" }
                                        else { (&format!("左から {count} 列")) }
                                    </option>
                                }
                            </select>
                        </div>

                        <div class="flex flex-col gap-1">
                            <label for="daywidth" class="text-xs font-medium text-slate-500">
                                (l.t("1日の幅"))
                            </label>
                            <select
                                id="daywidth"
                                name="day_width"
                                class="rounded-lg border border-slate-300 px-3 py-2"
                            >
                                for (width, label) in [(12u32, "狭い"), (18, "やや狭い"), (26, "標準"), (36, "広い")] {
                                    <option
                                        value=(width.to_string())
                                        selected=((width == data.day_width).then_some("selected"))
                                    >
                                        (label)
                                    </option>
                                }
                            </select>
                        </div>

                        <label class="flex items-center gap-2 pb-2.5 text-sm">
                            <input
                                type="checkbox"
                                name="quarters"
                                value="1"
                                checked=(data.quarters.then_some("checked"))
                                class="size-4"
                            >
                            (l.t("四半期の帯を出す"))
                        </label>

                        <label class="flex items-center gap-2 pb-2.5 text-sm">
                            <input
                                type="checkbox"
                                name="japanese_era"
                                value="1"
                                checked=(data.japanese_era.then_some("checked"))
                                class="size-4"
                            >
                            (l.t("年を和暦で表示する"))
                        </label>
                    </div>

                                        <button
                        class="w-fit rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                    >
                        (l.t("保存"))
                    </button>
                </form>
                }
            </section>

            // --- columns ----------------------------------------------------

            <section id="columns" class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">"列"</h2>
                <p class="mt-1 text-sm text-slate-500">
                    (l.t("表示・幅（px、空欄で自動）・並び順。タスク名は先頭で固定です。↑↓ を押すと、入力中の幅も一緒に保存されます。"))
                </p>

                if project.can_edit() {
                    <form
                        method="POST"
                        action=(("/projects/", &project.id, "/columns"))
                        class="mt-4 flex flex-col gap-3"
                    >
                        <ul class="flex flex-col divide-y divide-slate-100 border-y border-slate-100">
                            for (index, column) in columns.iter().enumerate() {
                                <li class="flex items-center gap-3 py-1.5 text-sm">
                                    <label class="flex w-44 items-center gap-2">
                                        <input
                                            type="checkbox"
                                            name=(("column_", &column.key))
                                            value="1"
                                            checked=(column.shown.then_some("checked"))
                                            disabled=((column.key == "name").then_some(""))
                                            class="size-4"
                                        >
                                        (&column.label)
                                    </label>

                                    <input
                                        name=(("width_", &column.key))
                                        value=(&column.width)
                                        inputmode="numeric"
                                        placeholder=(l.t("自動"))
                                        class="w-20 rounded-lg border border-slate-300 px-2 py-1 text-sm"
                                    >

                                    if column.key != "name" {
                                        <span class="ml-auto flex items-center gap-1">
                                            <button
                                                name="move"
                                                value=(("up:", &column.key))
                                                disabled=((index <= 1).then_some(""))
                                                class="rounded border border-slate-200 px-1.5 text-xs text-slate-500 hover:bg-slate-50 disabled:opacity-30"
                                            >
                                                "↑"
                                            </button>
                                            <button
                                                name="move"
                                                value=(("down:", &column.key))
                                                disabled=((index + 1 == columns.len()).then_some(""))
                                                class="rounded border border-slate-200 px-1.5 text-xs text-slate-500 hover:bg-slate-50 disabled:opacity-30"
                                            >
                                                "↓"
                                            </button>
                                        </span>
                                    }
                                </li>
                            }
                        </ul>

                        <button
                            class="w-fit rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                        >
                            (l.t("保存"))
                        </button>
                    </form>
                } else {
                    <ul class="mt-4 flex flex-col gap-1 text-sm text-slate-600">
                        for column in columns.iter().filter(|column| column.shown) {
                            <li>(&column.label)</li>
                        }
                    </ul>
                }
            </section>

            // --- statuses ---------------------------------------------------

            <details id="statuses" open=((open == "statuses").then_some("open")) class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <summary class="cursor-pointer text-lg font-semibold">
                    (l.t("ステータス"))
                    <span class="ml-2 text-sm font-normal text-slate-400">
                        (&format!("{} 種類", data.statuses.len()))
                    </span>
                </summary>
                <p class="mt-1 text-sm text-slate-500">
                    (l.t("名前・色・その状態が意味する進捗。進捗を空にすると、その状態では手入力のままになります。"))
                </p>

                if project.can_edit() {
                    <form
                        method="POST"
                        action=(("/projects/", &project.id, "/statuses"))
                        class="mt-4 flex flex-wrap items-end gap-3"
                    >
                        <div class="flex flex-col gap-1">
                            <label for="status-name" class="text-xs font-medium text-slate-500">
                                (l.t("名前"))
                            </label>
                            <input
                                id="status-name"
                                name="name"
                                required=""
                                placeholder=(l.t("レビュー中"))
                                class="w-40 rounded-lg border border-slate-300 px-3 py-2"
                            >
                        </div>

                        <div class="flex flex-col gap-1">
                            <label for="status-color" class="text-xs font-medium text-slate-500">
                                (l.t("色"))
                            </label>
                            <input
                                id="status-color"
                                name="color"
                                type="color"
                                value="#f1f5f9"
                                class="h-10 w-16 rounded-lg border border-slate-300 px-1"
                            >
                        </div>

                        <div class="flex flex-col gap-1">
                            <label for="status-percent" class="text-xs font-medium text-slate-500">
                                (l.t("進捗（任意）"))
                            </label>
                            <input
                                id="status-percent"
                                name="percent"
                                inputmode="numeric"
                                placeholder="100"
                                class="w-24 rounded-lg border border-slate-300 px-3 py-2"
                            >
                        </div>

                        <button
                            class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                        >
                            (l.t("追加・更新"))
                        </button>
                    </form>
                }

                <ul class="mt-6 divide-y divide-slate-100 border-t border-slate-100">
                    for (index, status) in data.statuses.iter().enumerate() {
                        <li class="flex items-center gap-4 py-2.5">
                            <span
                                class="rounded-full px-2.5 py-0.5 text-xs"
                                style=(("background:", &status.color))
                            >
                                (&status.name)
                            </span>
                            <span class="text-sm text-slate-500">
                                match status.percent {
                                    Some(percent) => (&format!("進捗 {percent}%")),
                                    None => "進捗は手入力",
                                }
                            </span>

                            if project.can_edit() {
                                // The menu offers them in this order, so the order
                                // is itself a setting.
                                <form
                                    method="POST"
                                    action=(("/projects/", &project.id, "/statuses/move"))
                                    class="ml-auto flex items-center gap-1"
                                >
                                    <input type="hidden" name="name" value=(&status.name)>
                                    <button
                                        name="direction"
                                        value="up"
                                        disabled=((index == 0).then_some(""))
                                        class="rounded border border-slate-200 px-1.5 text-xs text-slate-500 hover:bg-slate-50 disabled:opacity-30"
                                    >
                                        "↑"
                                    </button>
                                    <button
                                        name="direction"
                                        value="down"
                                        disabled=((index + 1 == data.statuses.len()).then_some(""))
                                        class="rounded border border-slate-200 px-1.5 text-xs text-slate-500 hover:bg-slate-50 disabled:opacity-30"
                                    >
                                        "↓"
                                    </button>
                                </form>

                                <form
                                    method="POST"
                                    action=(("/projects/", &project.id, "/statuses/remove"))
                                >
                                    <input type="hidden" name="name" value=(&status.name)>
                                    <button class="text-sm text-slate-400 hover:text-red-600">
                                        (l.t("削除"))
                                    </button>
                                </form>
                            }
                        </li>
                    }
                </ul>
            </details>

            // --- custom fields ----------------------------------------------

            <section id="fields" class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">"独自の項目"</h2>
                <p class="mt-1 text-sm text-slate-500">"基本の列のあとに並びます。"</p>

                if project.can_edit() {
                    <form
                        method="POST"
                        action=(("/projects/", &project.id, "/fields"))
                        class="mt-4 flex flex-wrap items-start gap-3"
                    >
                        <div class="flex flex-col gap-1">
                            <label for="field-label" class="text-xs font-medium text-slate-500">
                                (l.t("項目名"))
                            </label>
                            <input
                                id="field-label"
                                name="label"
                                required=""
                                placeholder=(l.t("製品"))
                                class="rounded-lg border border-slate-300 px-3 py-2"
                            >
                        </div>

                        <div class="flex flex-col gap-1">
                            <label for="field-kind" class="text-xs font-medium text-slate-500">
                                (l.t("種類"))
                            </label>
                            <select
                                id="field-kind"
                                name="kind"
                                class="rounded-lg border border-slate-300 px-3 py-2"
                            >
                                <option value="text">"フリー"</option>
                                <option value="select">"選択"</option>
                                <option value="suggest">"フリー＋選択"</option>
                                <option value="date">"日付"</option>
                                <option value="number">"数値"</option>
                            </select>
                        </div>

                        <button
                            class="mt-5 rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                        >
                            (l.t("追加"))
                        </button>
                    </form>
                }

                if data.fields.is_empty() {
                    <p class="mt-6 text-sm text-slate-400">"まだ登録がありません。"</p>
                } else {
                    <ul class="mt-6 flex flex-col divide-y divide-slate-100 border-t border-slate-100">
                        for field in &data.fields {
                            <li class="flex flex-col gap-3 py-3">
                                <div class="flex items-center gap-4">
                                    if project.can_edit() {
                                        // 名前は直せる。間違えたときに、消して作り
                                        // 直すしか道が無いと、入力済みの内容まで
                                        // 一緒に捨てることになる。
                                        <form
                                            method="POST"
                                            action=(("/projects/", &project.id, "/fields/rename"))
                                            class="flex items-center gap-2"
                                        >
                                            <input type="hidden" name="field_id" value=(&field.id)>
                                            <input
                                                name="label"
                                                value=(&field.label)
                                                required=""
                                                class="w-40 rounded-lg border border-slate-300 px-2.5 py-1.5 text-sm font-medium"
                                            >
                                            <button class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm hover:bg-slate-100">
                                                (l.t("名前を保存"))
                                            </button>
                                        </form>
                                    } else {
                                        <span class="font-medium">(&field.label)</span>
                                    }

                                    if project.can_edit() && !field.in_use {
                                        // 種類を変えられるのは、まだ何も入っていない
                                        // うちだけ。日付の入った列を数値にすると、
                                        // 読めない値がそのまま残る。
                                        <form
                                            method="POST"
                                            action=(("/projects/", &project.id, "/fields/kind"))
                                            class="flex items-center gap-2"
                                        >
                                            <input type="hidden" name="field_id" value=(&field.id)>
                                            <select
                                                name="kind"
                                                class="rounded-lg border border-slate-300 px-2 py-1.5 text-sm"
                                            >
                                                for (value, label) in [
                                                    ("text", "フリー"),
                                                    ("select", "選択"),
                                                    ("suggest", "フリー＋選択"),
                                                    ("date", "日付"),
                                                    ("number", "数値"),
                                                ] {
                                                    <option
                                                        value=(value)
                                                        selected=((field.kind == value).then_some("selected"))
                                                    >
                                                        (l.t(label))
                                                    </option>
                                                }
                                            </select>
                                            <button class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm hover:bg-slate-100">
                                                (l.t("種類を保存"))
                                            </button>
                                        </form>
                                    } else {
                                        <span class="rounded-full bg-slate-100 px-2.5 py-0.5 text-xs text-slate-600">
                                            (kind_label(&field.kind))
                                            if field.in_use {
                                                <span class="ml-1 text-slate-400">(l.t("（入力済み）"))</span>
                                            }
                                        </span>
                                    }

                                    if project.can_edit() {
                                        <form
                                            method="POST"
                                            action=(("/projects/", &project.id, "/fields/remove"))
                                            class="ml-auto"
                                        >
                                            <input type="hidden" name="field_id" value=(&field.id)>
                                            <button
                                                class="text-sm text-slate-400 hover:text-red-600"
                                                onclick="return confirm('この項目に入力した内容もすべて消えます。よろしいですか？')"
                                            >
                                                (l.t("削除"))
                                            </button>
                                        </form>
                                    }
                                </div>

                                // The master list, when the kind has one.
                                if field.kind == "select" || field.kind == "suggest" {
                                    <div class="flex flex-col gap-1.5 pl-1">
                                        for (index, option) in field.options.iter().enumerate() {
                                            option_row(
                                                project: &project,
                                                field: field,
                                                option: option,
                                                index: index,
                                                last: index + 1 == field.options.len(),
                                            )
                                        }

                                        if field.options.is_empty() {
                                            <p class="text-xs text-slate-400">"選択肢がありません。"</p>
                                        }

                                        if project.can_edit() {
                                            <form
                                                method="POST"
                                                action=(("/projects/", &project.id, "/fields/options"))
                                                class="flex flex-wrap items-center gap-2 pt-1"
                                            >
                                                <input type="hidden" name="field_id" value=(&field.id)>
                                                <input
                                                    name="value"
                                                    required=""
                                                    placeholder=(l.t("選択肢を追加"))
                                                    class="w-44 rounded-lg border border-slate-300 px-2.5 py-1.5 text-sm"
                                                >
                                                <label class="flex items-center gap-1 text-xs text-slate-500">
                                                    (l.t("文字"))
                                                    <input
                                                        name="color"
                                                        type="color"
                                                        value="#334155"
                                                        class="h-8 w-10 rounded border border-slate-300 px-0.5"
                                                    >
                                                </label>
                                                <label class="flex items-center gap-1 text-xs text-slate-500">
                                                    (l.t("背景"))
                                                    <input
                                                        name="background"
                                                        type="color"
                                                        value="#f1f5f9"
                                                        class="h-8 w-10 rounded border border-slate-300 px-0.5"
                                                    >
                                                </label>
                                                <button
                                                    class="rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-sm hover:bg-slate-50"
                                                >
                                                    (l.t("追加・更新"))
                                                </button>
                                            </form>
                                        }
                                    </div>
                                }
                            </li>
                        }
                    </ul>
                }
            </section>


            // --- assignees --------------------------------------------------

            <details id="assignees" open=((open == "assignees").then_some("open")) class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <summary class="cursor-pointer text-lg font-semibold">
                    (l.t("担当者"))
                    <span class="ml-2 text-sm font-normal text-slate-400">
                        (&format!("{} 人", data.assignees.len()))
                    </span>
                </summary>
                <p class="mt-1 text-sm text-slate-500">
                    (l.t("メンバーとタスクに入っている名前が並びます。ここに名前を足せば、アカウントの無い人も選べます。色は全員で共通なので「全体の設定」で決めます——同じ人が案件ごとに違う色だと、いくつも開いたときに読めなくなるためです。"))
                </p>

                if project.can_edit() {
                    <form
                        method="POST"
                        action=(("/projects/", &project.id, "/assignees"))
                        class="mt-4 flex flex-wrap items-center gap-2"
                    >
                        <input
                            name="name"
                            required=""
                            list="fugantt-assignees"
                            placeholder=(l.t("名前"))
                            class="w-40 rounded-lg border border-slate-300 px-3 py-2"
                        >
                        <button
                            class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                        >
                            (l.t("追加"))
                        </button>
                    </form>
                }

                if data.assignees.is_empty() {
                    <p class="mt-6 text-sm text-slate-400">"まだ誰も出てきていません。"</p>
                } else {
                    <ul class="mt-6 divide-y divide-slate-100 border-t border-slate-100">
                        for person in &data.assignees {
                            <li class="flex items-center gap-4 py-2.5">
                                <span
                                    class="rounded-full border border-slate-200 px-2.5 py-0.5 text-xs"
                                    style=((
                                        "color:",
                                        if person.color.is_empty() { "inherit" } else { &person.color },
                                        ";background:",
                                        if person.background.is_empty() {
                                            "transparent"
                                        } else {
                                            &person.background
                                        },
                                    ))
                                >
                                    (&person.name)
                                </span>

                                if person.color.is_empty() && person.background.is_empty() {
                                    <span class="text-xs text-slate-400">"色なし"</span>
                                }

                                if project.can_edit() {
                                    <form
                                        method="POST"
                                        action=(("/projects/", &project.id, "/assignees/remove"))
                                        class="ml-auto"
                                    >
                                        <input type="hidden" name="name" value=(&person.name)>
                                        <button class="text-sm text-slate-400 hover:text-red-600">
                                            (l.t("外す"))
                                        </button>
                                    </form>
                                }
                            </li>
                        }
                    </ul>
                }
            </details>

            // --- holidays ---------------------------------------------------

            <details id="holidays" open=((open == "holidays").then_some("open")) class="mt-8 rounded-xl border border-slate-200 bg-white p-6">
                <summary class="cursor-pointer text-lg font-semibold">
                    (l.t("祝日・休業日"))
                    <span class="ml-2 text-sm font-normal text-slate-400">
                        (&format!("{} 日", holidays.len()))
                    </span>
                </summary>
                <p class="mt-1 text-sm text-slate-500">
                    (l.t("土日と同じように網かけします。日数の計算は変わりません。日本の祝日は「全体の設定」に入れておくと、どのプロジェクトにも出ます。ここで扱うのは、この現場だけの違いです。"))
                </p>

                if project.can_edit() {
                    <form
                        method="POST"
                        action=(("/projects/", &project.id, "/holidays"))
                        class="mt-4 flex flex-wrap items-end gap-3"
                    >
                        <div class="flex flex-col gap-1">
                            <label for="holiday-date" class="text-xs font-medium text-slate-500">
                                (l.t("日付"))
                            </label>
                            <input
                                id="holiday-date"
                                name="date"
                                type="date"
                                required=""
                                class="rounded-lg border border-slate-300 px-3 py-2"
                            >
                        </div>

                        <div class="flex flex-1 flex-col gap-1">
                            <label for="holiday-name" class="text-xs font-medium text-slate-500">
                                (l.t("名称"))
                            </label>
                            <input
                                id="holiday-name"
                                name="name"
                                placeholder=(l.t("現場の休業日"))
                                class="w-full rounded-lg border border-slate-300 px-3 py-2"
                            >
                        </div>

                        <button
                            class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                        >
                            (l.t("このプロジェクトに追加"))
                        </button>
                    </form>
                }

                if !skipped.is_empty() {
                    <div class="mt-4 rounded-lg bg-amber-50 px-3 py-2">
                        <p class="text-xs text-amber-900">"全体の休みだが、このプロジェクトでは動く日"</p>
                        <ul class="mt-1 flex flex-wrap gap-2">
                            for date in &skipped {
                                <li class="flex items-center gap-1 rounded-full bg-white px-2.5 py-0.5 text-xs">
                                    <span class="font-mono tabular-nums">(date)</span>
                                    if project.can_edit() {
                                        <form
                                            method="POST"
                                            action=(("/projects/", &project.id, "/holidays/restore"))
                                        >
                                            <input type="hidden" name="date" value=(date)>
                                            <button class="text-slate-400 hover:text-blue-600">"戻す"</button>
                                        </form>
                                    }
                                </li>
                            }
                        </ul>
                    </div>
                }

                if holidays.is_empty() {
                    <p class="mt-6 text-sm text-slate-400">"まだ登録がありません。"</p>
                } else {
                    <ul class="mt-6 divide-y divide-slate-100 border-t border-slate-100">
                        for holiday in &holidays {
                            <li class="flex items-center gap-4 py-2.5">
                                <span class="w-28 font-mono text-sm tabular-nums">(&holiday.date)</span>
                                <span class="text-sm text-slate-600">(&holiday.name)</span>

                                if shared.contains(&holiday.date) {
                                    <span class="rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-500">
                                        (l.t("全体"))
                                    </span>
                                } else {
                                    <span class="rounded-full bg-blue-50 px-2 py-0.5 text-xs text-blue-700">
                                        (l.t("このプロジェクト"))
                                    </span>
                                }

                                if project.can_edit() {
                                    <form
                                        method="POST"
                                        action=(("/projects/", &project.id, "/holidays/remove"))
                                        class="ml-auto"
                                    >
                                        <input type="hidden" name="date" value=(&holiday.date)>
                                        <button class="text-sm text-slate-400 hover:text-red-600">
                                            // A shared day cannot be deleted from here.
                                            // What this project can record is that it
                                            // works through it.
                                            if shared.contains(&holiday.date) {
                                                (l.t("このプロジェクトでは働く"))
                                            } else {
                                                (l.t("削除"))
                                            }
                                        </button>
                                    </form>
                                }
                            </li>
                        }
                    </ul>
                }
            </details>

            // --- colours ----------------------------------------------------

            <section id="colours" class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">"バーの色"</h2>

                if !project.can_edit() {
                    <div class="mt-4 flex flex-wrap gap-5">
                        for (label, value) in [
                            ("予定", theme.bar.as_str()),
                            ("完了分", theme.done.as_str()),
                            ("集計行", theme.summary.as_str()),
                            ("遅延", theme.late.as_str()),
                            ("土曜", theme.saturday.as_str()),
                            ("日曜", theme.sunday.as_str()),
                            ("祝日", theme.holiday.as_str()),
                        ] {
                            <div class="flex items-center gap-2 text-sm">
                                <span
                                    class="size-5 rounded border border-slate-300"
                                    style=(("background:", value))
                                ></span>
                                (label)
                            </div>
                        }
                    </div>
                }

                if project.can_edit() {
                <form
                    method="POST"
                    action=(("/projects/", &project.id, "/colors"))
                    class="mt-4 flex flex-wrap items-end gap-5"
                >
                    colour_field(
                        name: "color_bar",
                        label: "予定",
                        value: theme.bar.as_str(),
                    )
                    colour_field(
                        name: "color_done",
                        label: "完了分",
                        value: theme.done.as_str(),
                    )
                    colour_field(
                        name: "color_actual",
                        label: "実施",
                        value: theme.actual.as_str(),
                    )
                    colour_field(
                        name: "color_summary",
                        label: "集計行",
                        value: theme.summary.as_str(),
                    )
                    colour_field(
                        name: "color_late",
                        label: "遅延",
                        value: theme.late.as_str(),
                    )
                    colour_field(
                        name: "color_saturday",
                        label: "土曜",
                        value: theme.saturday.as_str(),
                    )
                    colour_field(
                        name: "color_sunday",
                        label: "日曜",
                        value: theme.sunday.as_str(),
                    )
                    colour_field(
                        name: "color_holiday",
                        label: "祝日",
                        value: theme.holiday.as_str(),
                    )
                    colour_field(
                        name: "color_leave",
                        label: "休暇",
                        value: theme.leave.as_str(),
                    )
                    colour_field(
                        name: "color_today",
                        label: "今日",
                        value: theme.today.as_str(),
                    )
                    colour_field(
                        name: "color_wait",
                        label: "待ち",
                        value: theme.wait.as_str(),
                    )

                    <button
                        class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                    >
                        (l.t("保存"))
                    </button>
                </form>
                }
            </section>

            // --- access tokens ----------------------------------------------

            <section id="tokens" class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">(l.t("API トークン"))</h2>
                <p class="mt-1 text-sm text-slate-500">
                    (l.t("ブラウザ以外からこのプロジェクトだけを読み書きするための鍵です。書き出した JSON を読ませて、考えさせて、書き戻す——その往復に使います。"))
                </p>

                if !issued.is_empty() {
                    <div class="mt-4 rounded-lg border border-blue-200 bg-blue-50 p-4">
                        <p class="text-sm font-medium text-blue-900">
                            (l.t("いま作ったトークンです。この画面を離れると二度と出ません。"))
                        </p>
                        <code class="mt-2 block w-full break-all rounded-lg border border-blue-200 bg-white px-3 py-2 font-mono text-sm">
                            (&issued)
                        </code>
                    </div>
                }

                if project.is_owner() {
                    <form
                        method="POST"
                        action=(("/projects/", &project.id, "/tokens"))
                        class="mt-4 flex flex-wrap items-end gap-3"
                    >
                        <div class="flex flex-1 flex-col gap-1">
                            <label for="token-name" class="text-xs font-medium text-slate-500">
                                (l.t("用途"))
                            </label>
                            <input
                                id="token-name"
                                name="name"
                                placeholder=(l.t("週次の見直し"))
                                class="w-full rounded-lg border border-slate-300 px-3 py-2"
                            >
                        </div>

                        <div class="flex flex-col gap-1">
                            <label for="token-role" class="text-xs font-medium text-slate-500">
                                (l.t("権限"))
                            </label>
                            <select
                                id="token-role"
                                name="role"
                                class="rounded-lg border border-slate-300 px-3 py-2"
                            >
                                <option value="viewer">(l.t("読むだけ"))</option>
                                <option value="editor">(l.t("読み書き"))</option>
                            </select>
                        </div>

                        <button
                            class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                        >
                            (l.t("発行"))
                        </button>
                    </form>
                } else {
                    <p class="mt-4 text-sm text-slate-500">(l.t("トークンはオーナーが発行します。"))</p>
                }

                if tokens.is_empty() {
                    <p class="mt-6 text-sm text-slate-400">(l.t("まだありません。"))</p>
                } else {
                    <ul class="mt-6 divide-y divide-slate-100 border-t border-slate-100">
                        for token in &tokens {
                            <li class="flex items-center gap-4 py-2.5">
                                <span class="text-sm">
                                    if token.name.is_empty() { (l.t("（名前なし）")) } else { (&token.name) }
                                </span>
                                <span class="rounded-full bg-slate-100 px-2.5 py-0.5 text-xs text-slate-600">
                                    if token.role == "editor" { (l.t("読み書き")) } else { (l.t("読むだけ")) }
                                </span>
                                <span class="text-xs text-slate-400">
                                    match token.last_used {
                                        Some(at) => (&format!("{} {}", l.t("最終利用"), used_on(at))),
                                        None => (l.t("未使用")),
                                    }
                                </span>

                                if project.is_owner() {
                                    <form
                                        method="POST"
                                        action=(("/projects/", &project.id, "/tokens/remove"))
                                        class="ml-auto"
                                    >
                                        <input type="hidden" name="id" value=(&token.id)>
                                        <button class="text-sm text-slate-400 hover:text-red-600">
                                            (l.t("失効"))
                                        </button>
                                    </form>
                                }
                            </li>
                        }
                    </ul>
                }

                <details class="mt-6">
                    <summary class="cursor-pointer text-sm text-slate-500">(l.t("使い方"))</summary>
                    <pre class="mt-3 overflow-x-auto rounded-lg bg-slate-50 p-3 font-mono text-xs">(&usage)</pre>
                </details>
            </section>

            // --- members ----------------------------------------------------

            <section id="members" class="mt-6 rounded-xl border border-slate-200 bg-white p-6">
                <h2 class="text-lg font-semibold">"メンバー"</h2>

                if project.is_owner() {
                    <form
                        method="POST"
                        action=(("/projects/", &project.id, "/members"))
                        class="mt-4 flex flex-wrap items-end gap-3"
                    >
                        <div class="flex flex-1 flex-col gap-1">
                            <label for="member-email" class="text-xs font-medium text-slate-500">
                                (l.t("ユーザー名"))
                            </label>
                            // The account list is the whole set, so there is no
                            // reason to type a name — and typing invites typos.
                            <select
                                id="member-email"
                                name="email"
                                required=""
                                class="w-full rounded-lg border border-slate-300 px-3 py-2"
                            >
                                for account in &accounts {
                                    <option value=(&account.email)>
                                        if account.display_name.is_empty() {
                                            (&account.email)
                                        } else {
                                            (&format!("{}（{}）", account.display_name, account.email))
                                        }
                                    </option>
                                }
                            </select>
                        </div>

                        <div class="flex flex-col gap-1">
                            <label for="member-role" class="text-xs font-medium text-slate-500">
                                (l.t("権限"))
                            </label>
                            <select
                                id="member-role"
                                name="role"
                                class="rounded-lg border border-slate-300 px-3 py-2"
                            >
                                <option value="editor">"編集者"</option>
                                <option value="viewer">"閲覧者"</option>
                                <option value="owner">"オーナー"</option>
                            </select>
                        </div>

                        <button
                            class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                        >
                            (l.t("追加・更新"))
                        </button>
                    </form>
                }

                <ul class="mt-6 divide-y divide-slate-100 border-t border-slate-100">
                    for member in &members {
                        <li class="flex items-center gap-4 py-2.5">
                            <span class="text-sm">
                                if member.display_name.is_empty() {
                                    (&member.email)
                                } else {
                                    (&member.display_name)
                                }
                            </span>
                            // The address only when it adds something.
                            if !member.display_name.is_empty() {
                                <span class="text-xs text-slate-400">(&member.email)</span>
                            }
                            <span class="rounded-full bg-slate-100 px-2.5 py-0.5 text-xs text-slate-600">
                                (role_label(&member.role))
                            </span>

                            if project.is_owner() {
                                <form
                                    method="POST"
                                    action=(("/projects/", &project.id, "/members/remove"))
                                    class="ml-auto"
                                >
                                    <input type="hidden" name="user_id" value=(&member.user_id)>
                                    <button class="text-sm text-slate-400 hover:text-red-600">
                                        (l.t("外す"))
                                    </button>
                                </form>
                            }
                        </li>
                    }
                </ul>
            </section>
        </div>
    }
}

/// One swatch. The text field beside it is what makes the value copyable.
#[component]
async fn colour_field(name: &str, label: &str, value: &str) -> Result {
    view! {
        <div class="flex flex-col gap-1">
            <label for=(name) class="text-xs font-medium text-slate-500">(label)</label>
            <input
                id=(name)
                name=(name)
                type="color"
                value=(value)
                class="h-10 w-16 cursor-pointer rounded-lg border border-slate-300 bg-white p-1"
            >
        </div>
    }
}

/// A built-in column's name. Anything else is one of the project's own, which
/// carries its own label and never reaches here.
fn column_label(key: &str) -> &str {
    match key {
        // Named for what the grid calls them, `name` included: it used to fall
        // through to the last arm and label the task column as a note.
        "name" => "タスク",
        "waits" => "待ち",
        "targets" => "予定進捗",
        "late" => "遅延",
        "note" => "コメント",
        "start" => "予定開始",
        "end" => "予定終了",
        "actual_start" => "実施開始",
        "actual_end" => "実施終了",
        "start_variance" => "開始差異",
        "end_variance" => "終了差異",
        "days" => "予定日数",
        "actual_days" => "実作業日数",
        "progress" => "実進捗",
        "status" => "ステータス",
        "assignee" => "担当者",
        other => other,
    }
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "select" => "選択",
        "suggest" => "フリー＋選択",
        "date" => "日付",
        "number" => "数値",
        _ => "フリー",
    }
}

fn role_label(role: &str) -> &'static str {
    match role {
        // Deliberately not "administrator": that word is taken by the person who runs the
        // whole installation, and one row calling both things by one name is
        // how somebody ends up looking for the wrong page.
        "owner" => "オーナー",
        "viewer" => "閲覧者",
        _ => "編集者",
    }
}

/// One of the day-count switches.
#[component]
async fn count_switch(name: &str, label: &str, on: bool) -> Result {
    view! {
        <label class="flex items-center gap-2 text-sm">
            <input
                type="checkbox"
                name=(name)
                value="1"
                checked=(on.then_some("checked"))
                class="size-4"
            >
            (label)
        </label>
    }
}

/// One entry of a master list: how it looks, where it sits, and the way out.
#[component]
async fn option_row(
    project: &project::Project,
    field: &crate::domain::Field,
    option: &crate::domain::Option_,
    index: usize,
    last: bool,
) -> Result {
    let base = format!("/projects/{}/fields/options", project.id);
    let move_to = format!("{base}/move");
    let remove = format!("{base}/remove");

    view! {
        <div class="flex flex-wrap items-center gap-2">
            <span
                class="rounded-full border border-slate-200 px-2.5 py-0.5 text-xs"
                style=((
                    "color:", if option.color.is_empty() { "inherit" } else { &option.color },
                    ";background:",
                    if option.background.is_empty() { "transparent" } else { &option.background },
                ))
            >
                (&option.value)
            </span>

            if project.can_edit() {
                // Up and down rather than drag: two buttons work on every
                // machine and need no island of their own.
                <form method="POST" action=(&move_to) class="flex items-center gap-1">
                    <input type="hidden" name="field_id" value=(&field.id)>
                    <input type="hidden" name="value" value=(&option.value)>
                    <button
                        name="direction"
                        value="up"
                        disabled=((index == 0).then_some(""))
                        class="rounded border border-slate-200 px-1.5 text-xs text-slate-500 hover:bg-slate-50 disabled:opacity-30"
                    >
                        "↑"
                    </button>
                    <button
                        name="direction"
                        value="down"
                        disabled=(last.then_some(""))
                        class="rounded border border-slate-200 px-1.5 text-xs text-slate-500 hover:bg-slate-50 disabled:opacity-30"
                    >
                        "↓"
                    </button>
                </form>

                <form method="POST" action=(&remove)>
                    <input type="hidden" name="field_id" value=(&field.id)>
                    <input type="hidden" name="value" value=(&option.value)>
                    <button class="text-xs text-slate-400 hover:text-red-600">"削除"</button>
                </form>
            }
        </div>
    }
}

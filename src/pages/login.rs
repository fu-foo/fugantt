use topcoat::{
    Result,
    context::Cx,
    router::{error::redirect, page},
    view::view,
};

use crate::{auth::current_user, users};

/// Sign in or sign up. Both forms post to their own route in `auth`.
#[page("/login")]
async fn login_page(cx: &Cx) -> Result {
    if current_user(cx).await?.is_some() {
        return Err(redirect("/").into());
    }

    // The one moment self-registration is open: an installation with nobody in
    // it, where the first account has to come from somewhere. Everyone after
    // that is created by whoever holds that account.
    let may_register = users::none_yet(cx).await?;

    let l = crate::i18n::lang(cx).await;
    let rule = crate::app_settings::password_rule(cx).await.describe(l);

    view! {
        <div class=(
            if may_register {
                "mx-auto mt-10 grid max-w-2xl gap-6 sm:grid-cols-2"
            } else {
                "mx-auto mt-10 grid max-w-sm gap-6"
            }
        )>
            <section
                class="rounded-xl border border-slate-200 bg-white p-6 shadow-sm"
            >
                <h1 class="text-lg font-semibold">(l.t("ログイン"))</h1>

                <form method="POST" action="/login" class="mt-4 flex flex-col gap-3">
                    credential_fields(l: l, prefix: "login")
                    <button
                        class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-500"
                    >
                        (l.t("ログイン"))
                    </button>
                </form>
            </section>

            if may_register {
                <section
                    class="rounded-xl border border-slate-200 bg-white p-6 shadow-sm"
                >
                    <h1 class="text-lg font-semibold">
                        (l.t("最初のアカウントを作る"))
                    </h1>

                    <p class="mt-1 text-xs text-slate-500">
                        (l.t("最初に登録した人が管理者になり、以降のユーザーはその人が作ります。"))
                    </p>

                    <form method="POST" action="/register" class="mt-4 flex flex-col gap-3">

                        <label for="register-name" class="text-sm font-medium">(l.t("名前"))</label>
                        <input
                            id="register-name"
                            name="name"
                            required=""
                            placeholder=(l.t("山田 太郎"))
                            autocomplete="name"
                            class="rounded-lg border border-slate-300 px-3 py-2"
                        >
                        <p class="-mt-2 text-xs text-slate-500">
                            (l.t("画面にはこの名前が出ます。担当者としても選べます。"))
                        </p>

                        credential_fields(l: l, prefix: "register")
                        <p class="text-xs text-slate-500">(&l.about("パスワードは{}", &rule))</p>
                        <button
                            class="rounded-lg border border-slate-300 px-4 py-2 font-medium hover:bg-slate-50"
                        >
                            (l.t("登録"))
                        </button>
                    </form>
                </section>
            }
        </div>
    }
}

/// The email and password inputs, shared by both forms.
///
/// `prefix` keeps the `id`/`for` pairs unique across the two forms on the page.
#[topcoat::view::component]
async fn credential_fields(prefix: &str, l: crate::i18n::Lang) -> Result {
    let email_id = format!("{prefix}-email");
    let password_id = format!("{prefix}-password");

    view! {
        <label for=(&email_id) class="text-sm font-medium">(l.t("ユーザー名"))</label>
        <input
            id=(&email_id)
            name="email"
            required=""
            autocomplete="username"
            placeholder="yamada / yamada@example.com"
            class="rounded-lg border border-slate-300 px-3 py-2"
        >

        <label for=(&password_id) class="text-sm font-medium">(l.t("パスワード"))</label>
        <input
            id=(&password_id)
            name="password"
            type="password"
            required=""
            class="rounded-lg border border-slate-300 px-3 py-2"
        >
    }
}

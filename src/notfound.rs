//! The page for a request that did not work.
//!
//! Most of these come from inside a handler rather than from the router: the
//! address is a real route, and the project named in it is not there, or the
//! value posted to it was refused. Those answers used to be a line of plain
//! text on a blank page — `not found`, `bad request: …` — which reads as a
//! broken server rather than as something the reader can act on, and offers no
//! way back.
//!
//! Written as a layer because the pages this replaces are produced deep inside
//! handlers, where no layout runs. It is deliberately its own small page: the
//! drawer is about a project, and there may be no project here.
//!
//! A refusal that has somewhere better to go does not come through here. A
//! wrong password goes back to the login form, which is the box it was typed
//! in; this page is for the ones with nowhere to return to.

use topcoat::{
    context::CxBuilder,
    router::{
        Body, HeaderValue, IntoResponse, Layer, LayerFuture, Next, Path, Response, StatusCode,
        header, to_bytes,
    },
};

use crate::static_files;

pub struct NotFound;

impl Layer for NotFound {
    fn path(&self) -> &Path {
        Path::new("/")
    }

    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            // Read before the request is handed on: whether this is somebody
            // looking at a page, or a script reading the API. A script that
            // asked for JSON is not helped by a page telling it to go home.
            let (wants_page, language) = match cx.get::<http::request::Parts>() {
                Some(parts) => {
                    let accept = parts
                        .headers
                        .get("accept")
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or_default();

                    (
                        accept.contains("text/html") && !parts.uri.path().starts_with("/api/"),
                        crate::i18n::from_headers(&parts.headers),
                    )
                }
                None => (false, crate::i18n::Lang::Ja),
            };

            // A refused request arrives as an error, not as a response with a
            // status on it, and `?` here would carry it straight past this
            // layer to the plain-text rendering at the edge. Rendered first,
            // every refusal is a response like any other and can be answered
            // with a page.
            let response = match next.run(cx, body).await {
                Ok(response) => response,
                Err(error) => error.into_response(cx)?,
            };
            let status = response.status();

            if !wants_page || !(status.is_client_error() || status.is_server_error()) {
                return Ok(response);
            }

            if status == StatusCode::NOT_FOUND {
                return Ok(page(
                    language,
                    status,
                    language.t("そのページはありません"),
                    language
                        .t("消されたか、住所が違うか、見る権限が無いかのどれかです。")
                        .to_owned(),
                ));
            }

            // What the handler said, so the page can say it rather than a
            // shrug. Read to a limit: an error body is a sentence, and
            // anything longer than this is not one.
            let said = match to_bytes(response.into_body(), 8 * 1024).await {
                Ok(bytes) => String::from_utf8_lossy(&bytes).trim().to_owned(),
                Err(_) => String::new(),
            };

            // A server error's text is for whoever reads the log, not for
            // whoever is standing at the screen: it names the inside of the
            // program, and there is nothing to do about it from out here.
            let note = if status.is_server_error() || said.is_empty() {
                language
                    .t("こちら側の問題です。少し待ってから、もう一度お試しください。")
                    .to_owned()
            } else {
                said.strip_prefix("bad request: ").unwrap_or(&said).to_owned()
            };

            let title = if status.is_server_error() {
                language.t("うまく動きませんでした")
            } else {
                language.t("その操作はできませんでした")
            };

            Ok(page(language, status, title, note))
        })
    }
}

fn page(l: crate::i18n::Lang, status: StatusCode, title: &str, note: String) -> Response {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="icon" type="image/svg+xml" href="{favicon}">
<link rel="stylesheet" href="{tailwind}">
<link rel="stylesheet" href="{theme}">
</head>
<body class="flex min-h-screen items-center justify-center bg-slate-50 p-6">
<main class="w-full max-w-md text-center">
<h1 class="text-2xl font-bold tracking-tight">{title}</h1>
<p class="mt-3 text-sm text-slate-500">{note}</p>
<div class="mt-6 flex justify-center gap-3">
<button type="button" onclick="history.back()" class="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium hover:bg-slate-100">{back}</button>
<a href="/" class="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700">{home}</a>
</div>
</main>
</body>
</html>"#,
        lang = l.code(),
        title = escape(title),
        note = escape(&note),
        back = l.t("前の画面へ戻る"),
        home = l.t("プロジェクト一覧へ"),
        favicon = static_files::favicon(),
        tailwind = static_files::tailwind_css(),
        theme = static_files::theme_css(),
    );

    let mut response = Response::new(html.into());
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );

    response
}

/// The message goes into a page, and some of it came from a request.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

//! The page for an address that leads nowhere.
//!
//! Most of these come from inside a handler rather than from the router: the
//! address is a real route, and the project named in it is not there. That
//! answer used to be the two words `not found` on a blank page — which reads
//! as a broken server rather than as a mistyped link, and offers no way back.
//!
//! Written as a layer because the pages this replaces are produced deep inside
//! handlers, where no layout runs. It is deliberately its own small page: the
//! drawer is about a project, and there is no project here.

use topcoat::{
    context::CxBuilder,
    router::{Body, HeaderValue, Layer, LayerFuture, Next, Path, Response, StatusCode, header},
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

            let response = next.run(cx, body).await?;

            if response.status() != StatusCode::NOT_FOUND || !wants_page {
                return Ok(response);
            }

            Ok(page(language))
        })
    }
}

fn page(l: crate::i18n::Lang) -> Response {
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
<a href="/" class="mt-6 inline-block rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700">{home}</a>
</main>
</body>
</html>"#,
        lang = l.code(),
        title = l.t("そのページはありません"),
        note = l.t("消されたか、住所が違うか、見る権限が無いかのどれかです。"),
        home = l.t("プロジェクト一覧へ"),
        favicon = static_files::favicon(),
        tailwind = static_files::tailwind_css(),
        theme = static_files::theme_css(),
    );

    let mut response = Response::new(html.into());
    *response.status_mut() = StatusCode::NOT_FOUND;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );

    response
}

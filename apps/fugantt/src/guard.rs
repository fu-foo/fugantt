//! What every request goes through before any page or route sees it.
//!
//! Three things, kept together because each is a property of the whole server
//! rather than of one route — and a route that forgot one would be the hole:
//!
//! - **How much a request may send.** The framework reads a form or a JSON
//!   body whole, with no ceiling, and `/login` reads one before anybody has
//!   signed in. A single 300 MB request took the process from 40 MB to 900 MB.
//! - **Where a change may come from.** The session cookie is `SameSite=Lax`,
//!   which keeps other sites out but not other *ports* on the same host, nor a
//!   sibling subdomain — and a LAN server sharing a machine with something
//!   else is exactly how this gets run.
//! - **What the browser is told about the page.** No framing, no sniffing, and
//!   a content policy that keeps the page to its own scripts and images.

use http_body_util::Limited;
use topcoat::{
    context::CxBuilder,
    router::{
        Body, HeaderValue, IntoResponse, Layer, LayerFuture, Method, Next, Path, Response,
        StatusCode, header,
    },
};

/// What an ordinary request may carry. A cell, a form, a page of settings: a
/// few kilobytes. Your own CSS, the largest of them, stops at 20,000 characters.
const ORDINARY: u64 = 1024 * 1024;

/// A whole plan, as JSON. Ten thousand rows is about 5 MB.
const PLAN: u64 = 64 * 1024 * 1024;

/// A whole installation, as a database file.
const BACKUP: u64 = 1024 * 1024 * 1024;

pub struct Guard;

impl Layer for Guard {
    fn path(&self) -> &Path {
        Path::new("/")
    }

    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let Some(parts) = cx.get::<http::request::Parts>() else {
                return next.run(cx, body).await;
            };

            let language = crate::i18n::from_headers(&parts.headers);
            let wants_page = parts
                .headers
                .get(header::ACCEPT)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|accept| accept.contains("text/html"));

            if !same_origin(parts) {
                return Ok(secured(refuse(
                    wants_page,
                    language,
                    StatusCode::FORBIDDEN,
                    language.t("この画面からの操作ではないため、受け付けませんでした。"),
                )));
            }

            let limit = limit_for(&parts.method, parts.uri.path());
            let declared = parts
                .headers
                .get(header::CONTENT_LENGTH)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());

            // Said up front when the request says how big it is. One that does
            // not is cut off at the same size as it is read, below.
            if declared.is_some_and(|length| length > limit) {
                return Ok(secured(refuse(
                    wants_page,
                    language,
                    StatusCode::PAYLOAD_TOO_LARGE,
                    language.t("送られてきたデータが大きすぎます。"),
                )));
            }

            let body = Body::new(Limited::new(
                body,
                usize::try_from(limit).unwrap_or(usize::MAX),
            ));

            let response = match next.run(cx, body).await {
                Ok(response) => response,
                Err(error) => error.into_response(cx)?,
            };

            Ok(secured(response))
        })
    }
}

/// How much this request may send.
fn limit_for(method: &Method, path: &str) -> u64 {
    if *method != Method::POST {
        return ORDINARY;
    }

    if path == "/admin/restore" {
        return BACKUP;
    }

    let plan = (path.starts_with("/projects/") && path.ends_with("/import.json"))
        || (path.starts_with("/api/projects/") && path.ends_with("/document"));

    if plan { PLAN } else { ORDINARY }
}

/// Whether a request that changes something came from one of our own pages.
///
/// The browser says so itself in `Sec-Fetch-Site`, which it computes and no
/// page can set. Older browsers leave it out; for them the `Origin` is held
/// against the host the request was addressed to. With neither — `curl`, a
/// script — there is no browser to borrow a cookie from, and nothing to check.
///
/// A request with an API token is not riding anyone's cookie: whoever holds
/// the token is the one asking, wherever they ask from.
fn same_origin(parts: &http::request::Parts) -> bool {
    if matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return true;
    }

    let headers = &parts.headers;
    let text = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());

    if text("authorization").is_some_and(|value| value.starts_with("Bearer ")) {
        return true;
    }

    if let Some(site) = text("sec-fetch-site") {
        // `none` is the person themselves: a bookmark, the address bar.
        return site == "same-origin" || site == "none";
    }

    let Some(origin) = text("origin") else {
        return true;
    };

    // Behind a proxy the host the browser used arrives as X-Forwarded-Host.
    let host = text("x-forwarded-host")
        .and_then(|value| value.split(',').next())
        .or_else(|| text("host"))
        .map(str::trim)
        .unwrap_or_default();

    origin
        .split_once("://")
        .is_some_and(|(_, authority)| !host.is_empty() && authority.eq_ignore_ascii_case(host))
}

/// The page that says no, or a line of text for something that is not a browser.
fn refuse(
    wants_page: bool,
    language: crate::i18n::Lang,
    status: StatusCode,
    note: &str,
) -> Response {
    if wants_page {
        return crate::notfound::page(
            language,
            status,
            language.t("その操作はできませんでした"),
            note.to_owned(),
        );
    }

    let mut response = Response::new(note.to_owned().into());
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response
}

/// What the browser is told about every page.
fn secured(mut response: Response) -> Response {
    let headers = response.headers_mut();

    // Not inside anyone else's frame: a page that can be framed can be
    // clicked on by someone who cannot see it.
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("same-origin"));

    if let Ok(policy) = HeaderValue::from_str(&policy()) {
        headers.insert("content-security-policy", policy);
    }

    response
}

/// The content security policy.
///
/// Scripts and connections only from here: the island and its live updates.
/// Inline styles are allowed — the plan's colours are written onto the cells —
/// but images and fonts are not fetched from anywhere else, so a colour that
/// smuggles in a `url(…)` loads nothing.
///
/// Under `topcoat dev` the page also loads the reloader from the dev server,
/// and talks to it over a socket; that address is let in, and only then.
fn policy() -> String {
    let dev = std::env::var("TOPCOAT_DEV_URL").ok();
    let dev_script = dev
        .as_deref()
        .map(|url| format!(" {url}"))
        .unwrap_or_default();
    let dev_socket = dev
        .as_deref()
        .map(|url| {
            let socket = url
                .replacen("http://", "ws://", 1)
                .replacen("https://", "wss://", 1);
            format!(" {url} {socket}")
        })
        .unwrap_or_default();

    format!(
        "default-src 'self'; \
         script-src 'self'{dev_script}; \
         connect-src 'self'{dev_socket}; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data:; \
         font-src 'self' data:; \
         object-src 'none'; \
         base-uri 'self'; \
         form-action 'self'; \
         frame-ancestors 'none'"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: &str, headers: &[(&str, &str)]) -> http::request::Parts {
        let mut builder = http::Request::builder().method(method).uri("/projects");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(()).unwrap().into_parts().0
    }

    #[test]
    fn a_change_from_our_own_page_goes_through() {
        assert!(same_origin(&request(
            "POST",
            &[("sec-fetch-site", "same-origin")]
        )));
        assert!(same_origin(&request(
            "POST",
            &[
                ("origin", "http://127.0.0.1:1861"),
                ("host", "127.0.0.1:1861")
            ],
        )));
    }

    #[test]
    fn a_change_from_anywhere_else_is_refused() {
        // Another port on the same machine is the same *site*, and still not us.
        assert!(!same_origin(&request(
            "POST",
            &[("sec-fetch-site", "same-site")]
        )));
        assert!(!same_origin(&request(
            "POST",
            &[("sec-fetch-site", "cross-site")]
        )));
        assert!(!same_origin(&request(
            "POST",
            &[
                ("origin", "http://127.0.0.1:3000"),
                ("host", "127.0.0.1:1861")
            ],
        )));
        assert!(!same_origin(&request(
            "POST",
            &[("origin", "null"), ("host", "127.0.0.1:1861")]
        )));
    }

    #[test]
    fn reading_and_tokens_and_scripts_are_not_asked() {
        assert!(same_origin(&request(
            "GET",
            &[("sec-fetch-site", "cross-site")]
        )));
        assert!(same_origin(&request(
            "POST",
            &[
                ("sec-fetch-site", "cross-site"),
                ("authorization", "Bearer fug_x")
            ],
        )));
        // curl: no browser, no cookie to borrow.
        assert!(same_origin(&request("POST", &[])));
    }

    #[test]
    fn a_proxy_passes_the_host_the_browser_used() {
        assert!(same_origin(&request(
            "POST",
            &[
                ("origin", "https://plan.example.com"),
                ("host", "127.0.0.1:1861"),
                ("x-forwarded-host", "plan.example.com"),
            ],
        )));
    }

    #[test]
    fn only_a_whole_plan_or_a_backup_may_be_large() {
        assert_eq!(limit_for(&Method::POST, "/login"), ORDINARY);
        assert_eq!(limit_for(&Method::POST, "/projects/p/import.json"), PLAN);
        assert_eq!(limit_for(&Method::POST, "/api/projects/p/document"), PLAN);
        assert_eq!(limit_for(&Method::POST, "/admin/restore"), BACKUP);
    }
}

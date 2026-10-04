//! The session cookie, with an escape hatch for plain HTTP.
//!
//! Topcoat's own store always sets `Secure` and the `__Host-` prefix, which is
//! the right default: a session token must not travel in the clear. But a
//! browser refuses a `Secure` cookie from any `http://` origin except
//! localhost, so an office LAN — the exact place this tool is meant to live —
//! cannot log in at all.
//!
//! Setting `FUGANTT_ALLOW_HTTP=1` drops both, and the session token then
//! travels in the clear like everything else on that connection. It is a
//! deliberate trade, not a default: the variable has to be set on purpose, and
//! the server says so on the way up.

use std::{borrow::Cow, time::Duration};

use topcoat::{
    context::Cx,
    cookie::{Cookie, Cookies, SameSite},
    session::{Token, TokenStore, TokenStoreFuture},
};

/// Whether the operator has opted into serving over plain HTTP.
pub fn allows_http() -> bool {
    std::env::var("FUGANTT_ALLOW_HTTP").is_ok_and(|value| value == "1")
}

/// Carries the session token in a cookie, hardened unless HTTP is allowed.
pub struct SessionCookie {
    name: Cow<'static, str>,
    secure: bool,
}

impl SessionCookie {
    pub fn new() -> Self {
        let secure = !allows_http();

        Self {
            // The `__Host-` prefix requires `Secure`; a browser drops the
            // cookie outright if the two disagree, so the name moves with it.
            name: if secure {
                Cow::Borrowed("__Host-session")
            } else {
                Cow::Borrowed("session")
            },
            secure,
        }
    }

    fn jar<'cx>(&self, cx: &'cx Cx) -> impl Cookies + 'cx {
        topcoat::cookie::cookies(cx)
            .override_same_site(SameSite::Lax)
            .override_http_only(true)
            .override_secure(self.secure)
            .override_path("/")
    }
}

impl TokenStore for SessionCookie {
    fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>> {
        Box::pin(async move {
            let Some(cookie) = self.jar(cx).get(&self.name) else {
                return Ok(None);
            };

            Ok(Token::decode(cookie.value_trimmed()).ok())
        })
    }

    fn write<'a>(
        &'a self,
        cx: &'a Cx,
        token: Token,
        max_age: Duration,
    ) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            let max_age = topcoat::cookie::time::Duration::try_from(max_age)?;

            self.jar(cx)
                .override_max_age(max_age)
                .add(Cookie::new(self.name.clone(), token.encode()));

            Ok(())
        })
    }

    fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            self.jar(cx).remove(Cookie::new(self.name.clone(), ""));
            Ok(())
        })
    }
}

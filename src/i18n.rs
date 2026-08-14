//! The screen, in the language of whoever is reading it.
//!
//! The original wording stays Japanese and stays in the code. The lookup key is
//! that same wording, so reading the source needs no awareness of translation at
//! all, and anything untranslated comes out in Japanese rather than as a bare
//! key on the screen.
//!
//! The narrower answer wins:
//!
//! 1. the person's own choice (your settings → language)
//! 2. the installation's default
//! 3. `Accept-Language`, which the browser fills in from the operating system
//! 4. Japanese

mod en;

use topcoat::{context::Cx, router::headers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ja,
    En,
}

impl Lang {
    /// From what was stored. A value we cannot read means "not decided".
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "ja" => Some(Lang::Ja),
            "en" => Some(Lang::En),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::Ja => "ja",
            Lang::En => "en",
        }
    }

    /// How this reads in that language. Untranslated wording stays Japanese:
    /// far better than a blank, and a missing translation shows on the screen.
    pub fn t(self, ja: &'static str) -> &'static str {
        match self {
            Lang::Ja => ja,
            Lang::En => en::of(ja).unwrap_or(ja),
        }
    }
}

/// The language of whoever is reading this page.
pub async fn lang(cx: &Cx) -> Lang {
    if let Ok(Some(user)) = crate::auth::current_user(cx).await
        && let Some(chosen) = Lang::parse(&user.language)
    {
        return chosen;
    }

    match crate::app_settings::get(cx, "language").await {
        Ok(Some(value)) => match Lang::parse(&value) {
            Some(chosen) => chosen,
            // Both `auto` and anything unreadable defer to the browser.
            None => from_browser(cx),
        },
        _ => from_browser(cx),
    }
}

/// From `Accept-Language`, which the browser fills in from the operating system.
///
/// Quality values are ignored: the only question is whether the first entry is
/// Japanese, and in a list like `ja-JP,ja;q=0.9` the first entry is the language
/// that person actually reads.
fn from_browser(cx: &Cx) -> Lang {
    let header = headers(cx)
        .get("accept-language")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let first = header
        .split(',')
        .next()
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();

    if first.starts_with("ja") {
        Lang::Ja
    } else {
        Lang::En
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stored_choice_reads() {
        assert_eq!(Lang::parse("en"), Some(Lang::En));
        assert_eq!(Lang::parse(" ja "), Some(Lang::Ja));
        assert_eq!(Lang::parse("auto"), None);
        assert_eq!(Lang::parse(""), None);
    }

    #[test]
    fn a_missing_translation_stays_japanese() {
        assert_eq!(Lang::En.t("保存"), "Save");
        assert_eq!(Lang::Ja.t("保存"), "保存");
        assert_eq!(Lang::En.t("まだ訳していない言葉"), "まだ訳していない言葉");
    }
}

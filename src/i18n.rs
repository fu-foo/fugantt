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

    /// A day without its year: `8/5`, or `Aug 5`.
    ///
    /// English never gets `8/5`. That reads as the fifth of August to somebody
    /// in Boston and the eighth of May to somebody in Berlin, and a date nobody
    /// can be sure of is worse than a longer one.
    pub fn short_date(self, iso: &str) -> String {
        let mut parts = iso.split('-');
        let (_, month, day) = (parts.next(), parts.next(), parts.next());
        let (Some(month), Some(day)) = (month, day) else {
            return iso.to_owned();
        };

        let number: usize = month.trim_start_matches('0').parse().unwrap_or(0);
        let day = day.trim_start_matches('0');

        // Anything outside the twelve months is not a date this wrote, and a
        // half-formatted guess is worse to read than the stored form.
        let Some(name) = MONTHS_EN.get(number.wrapping_sub(1)) else {
            return iso.to_owned();
        };

        match self {
            Lang::Ja => format!("{number}/{day}"),
            Lang::En => format!("{name} {day}"),
        }
    }

    /// What sits between the two ends of a stretch of days.
    pub fn to_(self) -> &'static str {
        match self {
            Lang::Ja => "〜",
            Lang::En => "–",
        }
    }

    /// A moment, to the minute: `08/16 14:30`, or `Aug 16 14:30`.
    pub fn stamp(self, at: i64) -> String {
        self.zoned(
            at,
            match self {
                Lang::Ja => "%m/%d %H:%M",
                Lang::En => "%b %-d %H:%M",
            },
        )
    }

    /// A moment as the day it fell on: `2026/08/05`, or `2026-08-05`.
    pub fn day(self, at: i64) -> String {
        self.zoned(
            at,
            match self {
                Lang::Ja => "%Y/%m/%d",
                Lang::En => "%Y-%m-%d",
            },
        )
    }

    fn zoned(self, at: i64, how: &str) -> String {
        jiff::Timestamp::from_second(at)
            .map(|stamp| {
                stamp
                    .to_zoned(jiff::tz::TimeZone::system())
                    .strftime(how)
                    .to_string()
            })
            .unwrap_or_default()
    }
}

const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

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

    /// The reason this exists at all: `8/5` is two different days depending on
    /// who is reading it.
    #[test]
    fn a_short_date_is_never_ambiguous_in_english() {
        assert_eq!(Lang::Ja.short_date("2026-08-05"), "8/5");
        assert_eq!(Lang::En.short_date("2026-08-05"), "Aug 5");
        assert_eq!(Lang::Ja.short_date("2026-12-31"), "12/31");
        assert_eq!(Lang::En.short_date("2026-12-31"), "Dec 31");
    }

    /// Anything that is not a date comes back as it was, rather than as a
    /// half-formatted guess.
    #[test]
    fn what_is_not_a_date_is_left_alone() {
        assert_eq!(Lang::En.short_date(""), "");
        assert_eq!(Lang::En.short_date("いつか"), "いつか");
        assert_eq!(Lang::En.short_date("2026-13-01"), "2026-13-01");
    }

    #[test]
    fn a_moment_reads_as_the_language_writes_it() {
        // 2026-08-16 05:30 UTC. The zone is the machine's, so only the shape
        // is asserted: digits where digits go, and a month name in English.
        let ja = Lang::Ja.stamp(1_786_937_400);
        let en = Lang::En.stamp(1_786_937_400);

        assert!(ja.contains('/') && ja.contains(':'), "{ja}");
        assert!(en.chars().next().is_some_and(char::is_alphabetic), "{en}");
        assert!(Lang::Ja.day(1_786_937_400).contains('/'));
        assert!(Lang::En.day(1_786_937_400).contains('-'));
    }
}

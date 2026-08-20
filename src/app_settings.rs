//! Settings that belong to the installation rather than to one project.

use sqlx::Row;
use topcoat::{Result, context::Cx};

use crate::db;

/// What the header calls this thing.
pub const DEFAULT_NAME: &str = "fugantt";

/// The Japanese eras, written as `YYYY-MM-DD name`, newest first.
///
/// Hard-coding 令和 would work until the next era begins, and the next era is
/// announced about a month before it starts — far too late to ship a build to
/// every office running this. So the list is data.
pub const DEFAULT_ERAS: &str =
    "2019-05-01 令和\n1989-01-08 平成\n1926-12-25 昭和\n1912-07-30 大正\n1868-01-25 明治";

/// The password rule, held as numbers so it can be bent to a company's policy.
///
/// Requiring particular kinds of character is here in full knowledge that it
/// buys little security — it mostly produces `Password1!` over and over. A tool
/// used inside a company that cannot meet the written policy does not get used.
/// Kinds of character a password has to contain.
///
/// Counting "at least two of four" is another way to say this, but a written
/// policy usually names the kinds: letters, digits, a symbol. Held as a count,
/// the screen cannot say which ones are actually required.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Lower,
    Upper,
    Digit,
    Symbol,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Lower, Kind::Upper, Kind::Digit, Kind::Symbol];

    /// The name it is stored and posted under.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Lower => "lower",
            Kind::Upper => "upper",
            Kind::Digit => "digit",
            Kind::Symbol => "symbol",
        }
    }

    pub fn label(self, l: crate::i18n::Lang) -> &'static str {
        l.t(match self {
            Kind::Lower => "英小文字",
            Kind::Upper => "英大文字",
            Kind::Digit => "数字",
            Kind::Symbol => "記号",
        })
    }

    fn present_in(self, password: &str) -> bool {
        password.chars().any(|c| match self {
            Kind::Lower => c.is_ascii_lowercase(),
            Kind::Upper => c.is_ascii_uppercase(),
            Kind::Digit => c.is_ascii_digit(),
            // Anything outside ASCII — Japanese, for one — counts as a symbol.
            // The rule is not "letters and digits only".
            Kind::Symbol => !c.is_ascii_alphanumeric() && !c.is_whitespace(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordRule {
    /// Minimum length, counted in characters rather than bytes.
    pub min: usize,
    /// Kinds that must appear. Empty means none are required.
    pub kinds: Vec<Kind>,
    /// Passwords containing any of these are refused. One word per line.
    pub banned: Vec<String>,
}

impl Default for PasswordRule {
    fn default() -> Self {
        // Eight is short. Invalidating every password already handed out on the
        // day of an upgrade is worse, so the default stays where it was and
        // raising it is left to the setting.
        Self {
            min: 8,
            kinds: Vec::new(),
            banned: banned_words(DEFAULT_BANNED),
        }
    }
}

/// The words refused by default. Not a dictionary: these are the ones actually
/// tried first.
///
/// A company or product name differs by workplace, so the list can be added to.
pub const DEFAULT_BANNED: &str = "password\npass\n12345678\n123456789\nqwerty\nabc123\nadmin\nwelcome\nletmein\niloveyou\nfugantt\nパスワード";

/// One word per line; blank lines and surrounding space are dropped.
pub fn banned_words(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim().to_lowercase())
        .filter(|line| !line.is_empty())
        .collect()
}

impl PasswordRule {
    /// The wording shown on the form. A rule and a description that disagree
    /// leave a screen nobody can satisfy.
    pub fn describe(&self, l: crate::i18n::Lang) -> String {
        let names: Vec<&str> = self.kinds.iter().map(|kind| kind.label(l)).collect();

        // Built as a whole sentence in each language rather than as pieces
        // glued together: the order of "at least N characters" and what it must
        // contain is not the same in the two, and a sentence assembled out of
        // translated fragments reads like neither.
        match l {
            crate::i18n::Lang::Ja => {
                let kinds = if names.is_empty() {
                    String::new()
                } else {
                    format!("。{}を含む", names.join("・"))
                };
                let banned = if self.banned.is_empty() {
                    String::new()
                } else {
                    "。よくある語は使えません".to_owned()
                };

                format!("{}文字以上{}{}", self.min, kinds, banned)
            }
            crate::i18n::Lang::En => {
                let kinds = if names.is_empty() {
                    String::new()
                } else {
                    format!(", including {}", names.join(", "))
                };
                let banned = if self.banned.is_empty() {
                    String::new()
                } else {
                    ". Common words are refused".to_owned()
                };

                format!("{} characters or more{}{}", self.min, kinds, banned)
            }
        }
    }

    /// Why it was refused, or that it passed.
    pub fn check(&self, password: &str) -> std::result::Result<(), String> {
        // Counted in characters. Counted in bytes, a three-character Japanese
        // passphrase passes and seven ASCII characters do not, which is a rule
        // nobody asked for.
        if password.chars().count() < self.min {
            return Err(format!("パスワードは{}文字以上にしてください。", self.min));
        }

        // Refused if it contains one. Rejecting `password` while allowing
        // `password1` defends the wording of the rule and nothing else.
        let folded = password.to_lowercase();
        if let Some(word) = self
            .banned
            .iter()
            .find(|word| folded.contains(word.as_str()))
        {
            return Err(format!(
                "「{word}」を含むパスワードは、真っ先に試されるので使えません。"
            ));
        }

        let missing: Vec<&str> = self
            .kinds
            .iter()
            .filter(|kind| !kind.present_in(password))
            .map(|kind| kind.label(crate::i18n::Lang::Ja))
            .collect();

        if !missing.is_empty() {
            return Err(format!(
                "パスワードには{}を入れてください。",
                missing.join("と")
            ));
        }

        Ok(())
    }
}

/// The rule as stored, or the shipped default.
pub async fn password_rule(cx: &Cx) -> PasswordRule {
    let read = |value: Option<String>| value.and_then(|value| value.trim().parse::<usize>().ok());

    let fallback = PasswordRule::default();

    PasswordRule {
        min: read(get(cx, "password_min").await.ok().flatten())
            .unwrap_or(fallback.min)
            .clamp(4, 128),
        kinds: match get(cx, "password_kinds").await.ok().flatten() {
            Some(text) => parse_kinds(&text),
            None => fallback.kinds,
        },
        // Empty means nothing is refused. Without that, a workplace whose own
        // watchword happens to be on the default list can never use it.
        banned: match get(cx, "password_banned").await.ok().flatten() {
            Some(text) => banned_words(&text),
            None => fallback.banned,
        },
    }
}

/// A list like `lower,digit`. Words that mean nothing here are dropped.
pub fn parse_kinds(text: &str) -> Vec<Kind> {
    Kind::ALL
        .into_iter()
        .filter(|kind| text.split(',').any(|part| part.trim() == kind.key()))
        .collect()
}

/// The refused words, or the shipped default if nobody has touched them.
pub async fn banned_text(cx: &Cx) -> String {
    get(cx, "password_banned")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| DEFAULT_BANNED.to_owned())
}

pub async fn get(cx: &Cx, key: &str) -> Result<Option<String>> {
    let row = sqlx::query("SELECT value FROM app_settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(db::pool(cx))
        .await?;

    Ok(row.map(|row| row.get::<String, _>("value")))
}

pub async fn set(cx: &Cx, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(db::pool(cx))
    .await?;

    Ok(())
}

/// The name shown in the header, which a company may want to be its own.
pub async fn name(cx: &Cx) -> String {
    get(cx, "app_name")
        .await
        .ok()
        .flatten()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_NAME.to_owned())
}

/// The era table as stored, or the shipped default.
pub async fn eras_text(cx: &Cx) -> String {
    get(cx, "eras")
        .await
        .ok()
        .flatten()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_ERAS.to_owned())
}

/// Parses the era table, newest first.
///
/// A line the reader cannot make sense of is dropped rather than refused: one
/// typo must not take the year off every heading in the app.
pub fn parse_eras(text: &str) -> Vec<crate::domain::Era> {
    let mut eras: Vec<crate::domain::Era> = text
        .lines()
        .filter_map(|line| {
            let (from, name) = line.trim().split_once(char::is_whitespace)?;
            let from: jiff::civil::Date = from.trim().parse().ok()?;
            let name = name.trim();

            (!name.is_empty()).then(|| crate::domain::Era {
                from: from.to_string(),
                name: name.to_owned(),
            })
        })
        .collect();

    // Newest first, so the first match for a date is the era it falls in.
    eras.sort_by(|a, b| b.from.cmp(&a.from));
    eras
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_is_counted_in_characters() {
        let rule = PasswordRule {
            min: 8,
            kinds: vec![],
            banned: vec![],
        };

        // Eight characters, twenty-four bytes. Counting bytes let this pass.
        assert!(rule.check("あいうえおかきく").is_ok());
        assert!(rule.check("あいうえお").is_err());
        assert!(rule.check("abcdefgh").is_ok());
        assert!(rule.check("abcdefg").is_err());
    }

    #[test]
    fn the_asked_for_kinds_have_to_be_there() {
        let rule = PasswordRule {
            min: 8,
            kinds: vec![Kind::Digit, Kind::Symbol],
            banned: vec![],
        };

        assert!(rule.check("abcdefghij").is_err());
        assert!(rule.check("abcdefgh1").is_err());
        assert!(rule.check("abcdefgh1!").is_ok());
        // Japanese counts on the symbol side.
        assert!(rule.check("abcdefgh1あ").is_ok());
        // A kind that was not asked for need not be there.
        assert!(rule.check("ABCDEFGH1!").is_ok());
    }

    #[test]
    fn no_kinds_asked_for_means_no_kinds_checked() {
        let rule = PasswordRule {
            min: 8,
            kinds: vec![],
            banned: vec![],
        };

        assert!(rule.check("aaaaaaaa").is_ok());
    }

    #[test]
    fn the_kinds_survive_the_round_trip() {
        assert_eq!(parse_kinds("upper,digit"), vec![Kind::Upper, Kind::Digit]);
        assert!(parse_kinds("").is_empty());
        // An older numeric setting reads as "none required".
        assert!(parse_kinds("2").is_empty());
    }

    #[test]
    fn the_rule_says_itself_out_loud() {
        assert_eq!(
            PasswordRule {
                min: 12,
                kinds: vec![],
                banned: vec![]
            }
            .describe(crate::i18n::Lang::Ja),
            "12文字以上"
        );
        assert_eq!(
            PasswordRule {
                min: 12,
                kinds: vec![Kind::Digit, Kind::Symbol],
                banned: vec!["password".to_owned()],
            }
            .describe(crate::i18n::Lang::Ja),
            "12文字以上。数字・記号を含む。よくある語は使えません"
        );
    }

    #[test]
    fn a_common_word_is_refused_even_dressed_up() {
        let rule = PasswordRule::default();

        assert!(rule.check("password").is_err());
        assert!(rule.check("MyPassword1").is_err());
        assert!(rule.check("12345678").is_err());
        assert!(rule.check("かきくけこさしす").is_ok());
    }

    /// An empty list refuses nothing: a team's own watchword may well be on
    /// the default list.
    #[test]
    fn an_empty_list_bans_nothing() {
        let rule = PasswordRule {
            min: 8,
            kinds: vec![],
            banned: banned_words("  \n \n"),
        };

        assert!(rule.check("password").is_ok());
    }

    #[test]
    fn the_default_table_reads() {
        let eras = parse_eras(DEFAULT_ERAS);

        assert_eq!(eras.len(), 5);
        assert_eq!(eras[0].name, "令和");
        assert_eq!(eras[0].from, "2019-05-01");
        assert_eq!(eras[2].name, "昭和");
        assert_eq!(eras[4].name, "明治");
    }

    /// One bad line must not cost the whole table.
    #[test]
    fn nonsense_lines_are_dropped() {
        let eras = parse_eras("2019-05-01 令和\nこれは元号ではない\n\n1989-01-08 平成");

        assert_eq!(eras.len(), 2);
    }
}

//! Reading what somebody typed into a cell.
//!
//! The grid sends the text as it was typed — `8/5`, `0805`, `8/17〜8/21 他部署`
//! — and reading it is the server's job, so that every host reads it the same
//! way. Nothing here knows what language the person reads: a refusal says what
//! kind it is, and the host puts it into words.
//!
//! "Today" is handed in. A date with the year left out means this year, and
//! which year that is depends on who is asking and when.

use jiff::civil::Date;

/// Why a cell's text could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextError {
    /// Not `#rrggbb`.
    Colour,
    /// 予定進捗 was not a date and a percentage.
    TargetShape,
    /// 予定進捗 named a percentage outside 0–100.
    TargetPercent,
    /// 待ち was not a range.
    WaitShape,
    /// A day inside a 待ち or 予定進捗 could not be read.
    WaitDate,
    /// A date cell could not be read.
    Date,
}

/// Folds full-width digits and separators onto their ASCII forms.
///
/// A Japanese keyboard left in kana or full-width mode turns "2026-09-01" into
/// "２０２６－０９－０１", which is the same date typed the same way and should
/// not be refused for it.
pub fn normalize_width(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap_or(c),
            'ａ'..='ｚ' => char::from_u32(c as u32 - 'ａ' as u32 + 'a' as u32).unwrap_or(c),
            'Ａ'..='Ｚ' => char::from_u32(c as u32 - 'Ａ' as u32 + 'A' as u32).unwrap_or(c),
            '－' | 'ー' | '−' | '‐' => '-',
            '／' => '/',
            '．' => '.',
            '％' => '%',
            '　' => ' ',
            _ => c,
        })
        .collect()
}

/// A colour, or nothing.
///
/// Only `#rrggbb`, and only lower case: the value goes straight into a style
/// attribute, and a colour is the one kind of user input that has no reason to
/// be anything but six hex digits.
pub fn colour(value: &str) -> Result<String, TextError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }

    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(TextError::Colour);
    }

    Ok(format!("#{}", hex.to_ascii_lowercase()))
}

/// Reads a cell of 予定進捗 into the stored form: `YYYY-MM-DD/PERCENT` a line.
///
/// Written the way a person would say it — `8/20 30%, 8/28 100%` — in either
/// width, with the percent sign optional. Each line is one promise: by this
/// date, this much. Nothing is read into the gap between two of them.
pub fn targets(value: &str, today: Date) -> Result<String, TextError> {
    let text = normalize_width(value);
    let mut stored: Vec<(Date, i64)> = Vec::new();

    for part in text.split(['\n', ',', '、']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (date, percent) = part
            .rsplit_once([' ', '/', '\t'])
            .ok_or(TextError::TargetShape)?;

        let date = wait_date(date.trim(), today)?;
        let percent: i64 = percent
            .trim()
            .trim_end_matches(['%', '％'])
            .trim()
            .parse()
            .map_err(|_| TextError::TargetShape)?;

        if !(0..=100).contains(&percent) {
            return Err(TextError::TargetPercent);
        }

        // The same date twice is one promise revised, not two.
        stored.retain(|(had, _)| *had != date);
        stored.push((date, percent));
    }

    stored.sort_by_key(|(date, _)| *date);

    Ok(stored
        .iter()
        .map(|(date, percent)| format!("{date}/{percent}"))
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Reads a cell of waiting periods into the stored form.
///
/// People write ranges every which way — `8/17〜8/21`, `2026-08-17 - 2026-08-21`
/// — and in whichever width the IME was in. A range with no end (`9/1〜`) is one
/// that has not finished: the days keep counting until it does. Anything after
/// the range is the reason, which is a note that happens to be worth counting.
pub fn waits(value: &str, today: Date) -> Result<String, TextError> {
    let text = normalize_width(value);
    let mut stored: Vec<String> = Vec::new();

    for part in text.split(['\n', ',', '、']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (range, reason) = split_reason(part);
        let range = range.replace(' ', "");

        let (from, to) = range
            .split_once(['〜', '~', '～'])
            .or_else(|| range.split_once(" - "))
            .or_else(|| split_dash_range(&range))
            .ok_or(TextError::WaitShape)?;

        let from = wait_date(from, today)?;
        let to = to.trim();

        // No end written means it is still waiting.
        if to.is_empty() {
            stored.push(with_reason(&format!("{from}/"), &reason));
            continue;
        }

        let to = wait_date(to, today)?;
        let (from, to) = if to < from { (to, from) } else { (from, to) };

        stored.push(with_reason(&format!("{from}/{to}"), &reason));
    }

    Ok(stored.join("\n"))
}

/// Splits `8/17〜8/21 他部署` into its range and its reason.
///
/// Token by token: the range is however many words at the front are made only
/// of date characters, and everything after them is the reason. That reads
/// `8/17 〜 8/21 他部署` and `8/17〜8/21 3社` the same way a person does.
fn split_reason(part: &str) -> (String, String) {
    let is_date_ish = |text: &str| {
        !text.is_empty()
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || "-/.〜~～年月日".contains(c))
    };

    let tokens: Vec<&str> = part.split_whitespace().collect();
    let range = tokens
        .iter()
        .take_while(|token| is_date_ish(token))
        .count()
        .max(1);

    (tokens[..range].join(" "), tokens[range..].join(" "))
}

fn with_reason(range: &str, reason: &str) -> String {
    if reason.is_empty() {
        range.to_owned()
    } else {
        // Newline separated above, so a colon is free to mark the reason.
        format!("{range}:{}", reason.replace([':', '\n'], " ").trim())
    }
}

/// `2026-08-17 - 2026-08-21`, where the separator is the same character the
/// dates themselves use. Splitting on the dash surrounded by spaces is the only
/// reading that cannot be confused with the date's own dashes.
fn split_dash_range(part: &str) -> Option<(&str, &str)> {
    part.split_once(" - ")
        .or_else(|| part.split_once('/').filter(|(from, _)| from.contains('-')))
}

/// A day inside a waiting range, read the same way as any other date cell.
fn wait_date(text: &str, today: Date) -> Result<Date, TextError> {
    flexible_date(text, today).ok_or(TextError::WaitDate)
}

/// An empty cell clears the date; anything else must be a real calendar day.
pub fn date(value: &str, today: Date) -> Result<Option<String>, TextError> {
    let value = normalize_width(value.trim());
    let value = value.trim_end_matches('%').trim();

    if value.is_empty() {
        return Ok(None);
    }

    let date = flexible_date(value, today).ok_or(TextError::Date)?;

    Ok(Some(date.to_string()))
}

/// A date, however somebody typed it.
///
/// Nobody reaches for the hyphens: on a numeric keypad `20260805`, `0805`,
/// `805` and `05` are the fast ways to say a day, and `8/5` is how it gets
/// written by hand. All of them mean a day, so all of them are accepted.
///
/// Bare digits are read by how many there are: one or two are a day this
/// month, three or four a day this year, eight a whole date. Nothing is
/// carried forward — `3` on the 28th of December is the third of December, in
/// the past. Most of what gets typed into 実施開始 is in the past, and a
/// reading that helpfully moved it on would make yesterday impossible to say.
pub fn flexible_date(value: &str, today: Date) -> Option<Date> {
    // 年, 月 and 日 read as separators; a trailing 日 is only punctuation.
    let value = normalize_width(value)
        .trim()
        .replace(['/', '.', '年', '月'], "-")
        .replace('日', "");
    let value = value.trim_end_matches('-').to_owned();
    let year = today.year();
    let month = today.month();

    if value.chars().all(|c| c.is_ascii_digit()) {
        // Only the odd lengths that mean something. Padding any odd length
        // would let seven digits fall into the whole-date reading and come
        // back as the year 26.
        let padded = match value.len() {
            1 | 3 => format!("0{value}"),
            _ => value.clone(),
        };

        return match padded.len() {
            8 => format!("{}-{}-{}", &padded[..4], &padded[4..6], &padded[6..])
                .parse()
                .ok(),
            4 => format!("{year}-{}-{}", &padded[..2], &padded[2..])
                .parse()
                .ok(),
            2 => format!("{year}-{month:02}-{padded}").parse().ok(),
            _ => None,
        };
    }

    let parts: Vec<&str> = value.split('-').filter(|part| !part.is_empty()).collect();

    let text = match parts.as_slice() {
        [month, day] => format!("{year}-{month:0>2}-{day:0>2}"),
        [year, month, day] => format!("{year:0>4}-{month:0>2}-{day:0>2}"),
        _ => return None,
    };

    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date as day;

    use super::*;

    #[test]
    fn a_date_is_read_however_it_was_typed() {
        let today = day(2026, 9, 15);

        for typed in [
            "2026-08-05",
            "20260805",
            "2026/8/5",
            "2026年8月5日",
            "8/5",
            "0805",
            "805",
            "８／５",
        ] {
            assert_eq!(
                flexible_date(typed, today),
                Some(day(2026, 8, 5)),
                "{typed}"
            );
        }

        // One or two digits are a day of this month.
        assert_eq!(flexible_date("5", today), Some(day(2026, 9, 5)));
        assert_eq!(flexible_date("05", today), Some(day(2026, 9, 5)));

        for typed in ["", "abc", "2026-02-30", "1234567", "13/1"] {
            assert_eq!(flexible_date(typed, today), None, "{typed}");
        }
    }

    /// The year and the month left out are today's — whatever today is. Read
    /// on the last day of a year and on the first day of the next, the same
    /// keys mean days a year apart.
    #[test]
    fn what_is_left_out_is_taken_from_the_day_it_is_read() {
        let old_year = day(2026, 12, 31);
        let new_year = day(2027, 1, 1);

        assert_eq!(flexible_date("1/5", old_year), Some(day(2026, 1, 5)));
        assert_eq!(flexible_date("1/5", new_year), Some(day(2027, 1, 5)));
        assert_eq!(flexible_date("3", old_year), Some(day(2026, 12, 3)));
        assert_eq!(flexible_date("3", new_year), Some(day(2027, 1, 3)));
        // Nothing is carried forward: the 31st of a month that has thirty days
        // is not a day.
        assert_eq!(flexible_date("31", day(2026, 9, 15)), None);
    }

    #[test]
    fn an_empty_cell_clears_the_date() {
        let today = day(2026, 9, 15);

        assert_eq!(date("", today), Ok(None));
        assert_eq!(date("  ", today), Ok(None));
        assert_eq!(date("8/5", today), Ok(Some("2026-08-05".to_owned())));
        assert_eq!(date("nope", today), Err(TextError::Date));
    }

    #[test]
    fn waits_are_ranges_with_an_optional_reason() {
        let today = day(2026, 9, 15);

        assert_eq!(
            waits("8/17〜8/21", today),
            Ok("2026-08-17/2026-08-21".to_owned())
        );
        assert_eq!(
            waits("8/17〜8/21 他部署, 9/1〜", today),
            Ok("2026-08-17/2026-08-21:他部署\n2026-09-01/".to_owned())
        );
        // Written backwards is still that range.
        assert_eq!(
            waits("8/21〜8/17", today),
            Ok("2026-08-17/2026-08-21".to_owned())
        );
        assert_eq!(waits("", today), Ok(String::new()));
        assert_eq!(waits("8/17", today), Err(TextError::WaitShape));
        assert_eq!(waits("x〜8/17", today), Err(TextError::WaitDate));
    }

    #[test]
    fn targets_are_a_date_and_a_percentage() {
        let today = day(2026, 9, 15);

        assert_eq!(
            targets("2026-08-20 30%", today),
            Ok("2026-08-20/30".to_owned())
        );
        assert_eq!(
            targets("8/28 100, 8/20 30％", today),
            Ok("2026-08-20/30\n2026-08-28/100".to_owned())
        );
        // The same date twice is one promise revised.
        assert_eq!(
            targets("8/20 30, 8/20 40", today),
            Ok("2026-08-20/40".to_owned())
        );
        // Nothing to split a date from a percentage on.
        assert_eq!(targets("0820", today), Err(TextError::TargetShape));
        assert_eq!(targets("8/20 abc", today), Err(TextError::TargetShape));
        assert_eq!(targets("8/20 101", today), Err(TextError::TargetPercent));
        assert_eq!(targets("x 30", today), Err(TextError::WaitDate));
    }

    #[test]
    fn a_colour_is_six_hex_digits_or_nothing() {
        assert_eq!(colour(""), Ok(String::new()));
        assert_eq!(colour("#B91C1C"), Ok("#b91c1c".to_owned()));
        assert_eq!(colour("b91c1c"), Ok("#b91c1c".to_owned()));
        assert_eq!(colour("#fff"), Err(TextError::Colour));
        assert_eq!(colour("red"), Err(TextError::Colour));
    }
}

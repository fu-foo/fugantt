//! Japanese public holidays.
//!
//! Typing sixteen dates a year by hand is the kind of work a schedule tool
//! should not create. The rules are in 国民の祝日に関する法律, and they are
//! rules rather than a list: two of the days move with the equinoxes, four are
//! "the nth Monday", and two more are produced by holidays landing badly.
//!
//! Valid from 2020, when 天皇誕生日 moved to 2月23日 and 体育の日 became
//! スポーツの日. Earlier years would need the older names and dates, and no
//! plan in this tool reaches back that far.

use jiff::civil::{Date, Weekday};

/// Every public holiday in `year`, in date order, with its name.
pub fn japanese(year: i16) -> Vec<(Date, &'static str)> {
    let mut days: Vec<(Date, &'static str)> = Vec::new();

    let mut fixed = |month: i8, day: i8, name: &'static str| {
        if let Ok(date) = Date::new(year, month, day) {
            days.push((date, name));
        }
    };

    fixed(1, 1, "元日");
    fixed(2, 11, "建国記念の日");
    fixed(2, 23, "天皇誕生日");
    fixed(4, 29, "昭和の日");
    fixed(5, 3, "憲法記念日");
    fixed(5, 4, "みどりの日");
    fixed(5, 5, "こどもの日");
    fixed(8, 11, "山の日");
    fixed(11, 3, "文化の日");
    fixed(11, 23, "勤労感謝の日");

    // The "happy Monday" holidays: the nth Monday of the month.
    for (month, nth, name) in [
        (1, 2, "成人の日"),
        (7, 3, "海の日"),
        (9, 3, "敬老の日"),
        (10, 2, "スポーツの日"),
    ] {
        if let Some(date) = nth_monday(year, month, nth) {
            days.push((date, name));
        }
    }

    if let Some(date) = equinox(year, Season::Spring) {
        days.push((date, "春分の日"));
    }
    if let Some(date) = equinox(year, Season::Autumn) {
        days.push((date, "秋分の日"));
    }

    days.sort_by_key(|(date, _)| *date);

    add_substitutes(&mut days);
    add_citizens_holidays(&mut days);

    days.sort_by_key(|(date, _)| *date);
    days
}

fn nth_monday(year: i16, month: i8, nth: u32) -> Option<Date> {
    let first = Date::new(year, month, 1).ok()?;

    // Days from the 1st to that month's first Monday.
    let offset = (Weekday::Monday.to_monday_zero_offset()
        - first.weekday().to_monday_zero_offset())
    .rem_euclid(7);

    let day = 1 + i64::from(offset) + i64::from(nth - 1) * 7;

    first
        .checked_add(jiff::Span::new().days(day - 1))
        .ok()
        .filter(|date| date.month() == month)
}

enum Season {
    Spring,
    Autumn,
}

/// The equinox days, from the approximation the Cabinet Office publishes.
///
/// Good for 1980–2099, which is all this tool will ever be asked about.
fn equinox(year: i16, season: Season) -> Option<Date> {
    if !(1980..=2099).contains(&year) {
        return None;
    }

    let base = match season {
        Season::Spring => 20.8431,
        Season::Autumn => 23.2488,
    };

    let years = f64::from(year - 1980);
    let day = (base + 0.242_194 * years - (years / 4.0).floor()).floor() as i8;
    let month = match season {
        Season::Spring => 3,
        Season::Autumn => 9,
    };

    Date::new(year, month, day).ok()
}

/// 振替休日: a holiday on a Sunday moves to the next day that is not itself one.
fn add_substitutes(days: &mut Vec<(Date, &'static str)>) {
    let existing: Vec<Date> = days.iter().map(|(date, _)| *date).collect();
    let mut extra = Vec::new();

    for date in existing.iter().filter(|d| d.weekday() == Weekday::Sunday) {
        let mut candidate = *date;

        // May 3rd lands on a Sunday behind two more holidays, so this walks
        // rather than simply adding a day.
        loop {
            let Ok(next) = candidate.checked_add(jiff::Span::new().days(1)) else {
                break;
            };
            candidate = next;

            if !existing.contains(&candidate) && !extra.iter().any(|(d, _)| *d == candidate) {
                extra.push((candidate, "振替休日"));
                break;
            }
        }
    }

    days.extend(extra);
}

/// 国民の休日: a single ordinary day held between two holidays becomes one too.
///
/// In practice this is the Tuesday of シルバーウィーク, when 敬老の日 and
/// 秋分の日 fall two days apart.
fn add_citizens_holidays(days: &mut Vec<(Date, &'static str)>) {
    let existing: Vec<Date> = days.iter().map(|(date, _)| *date).collect();
    let mut extra = Vec::new();

    for date in &existing {
        let Ok(gap) = date.checked_add(jiff::Span::new().days(1)) else {
            continue;
        };
        let Ok(after) = date.checked_add(jiff::Span::new().days(2)) else {
            continue;
        };

        if existing.contains(&after)
            && !existing.contains(&gap)
            && gap.weekday() != Weekday::Sunday
        {
            extra.push((gap, "国民の休日"));
        }
    }

    days.extend(extra);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(year: i16) -> Vec<(String, &'static str)> {
        japanese(year)
            .into_iter()
            .map(|(date, name)| (date.to_string(), name))
            .collect()
    }

    /// The whole of 2026, checked against the Cabinet Office's published list.
    #[test]
    fn twenty_twenty_six_matches_the_published_calendar() {
        assert_eq!(
            names(2026),
            [
                ("2026-01-01".to_owned(), "元日"),
                ("2026-01-12".to_owned(), "成人の日"),
                ("2026-02-11".to_owned(), "建国記念の日"),
                ("2026-02-23".to_owned(), "天皇誕生日"),
                ("2026-03-20".to_owned(), "春分の日"),
                ("2026-04-29".to_owned(), "昭和の日"),
                ("2026-05-03".to_owned(), "憲法記念日"),
                ("2026-05-04".to_owned(), "みどりの日"),
                ("2026-05-05".to_owned(), "こどもの日"),
                ("2026-05-06".to_owned(), "振替休日"),
                ("2026-07-20".to_owned(), "海の日"),
                ("2026-08-11".to_owned(), "山の日"),
                ("2026-09-21".to_owned(), "敬老の日"),
                ("2026-09-22".to_owned(), "国民の休日"),
                ("2026-09-23".to_owned(), "秋分の日"),
                ("2026-10-12".to_owned(), "スポーツの日"),
                ("2026-11-03".to_owned(), "文化の日"),
                ("2026-11-23".to_owned(), "勤労感謝の日"),
            ]
        );
    }

    /// When 5月3日 falls on a Sunday the run stretches to 5月6日: simply taking
    /// the next day would land on 5月4日, which is already a holiday.
    #[test]
    fn a_substitute_skips_over_the_holidays_behind_it() {
        let days = names(2026);

        assert!(days.contains(&("2026-05-06".to_owned(), "振替休日")), "{days:?}");
    }

    /// Two days between 敬老の日 and 秋分の日 make the シルバーウィーク run.
    #[test]
    fn silver_week_appears_only_when_the_gap_is_one_day() {
        assert!(names(2026).contains(&("2026-09-22".to_owned(), "国民の休日")));
        // 2027 has 敬老の日 on 9/20 and 秋分の日 on 9/23, too far apart for one.
        assert!(!names(2027).iter().any(|(_, name)| *name == "国民の休日"));
    }

    #[test]
    fn the_equinoxes_move_with_the_year() {
        assert!(names(2025).contains(&("2025-03-20".to_owned(), "春分の日")));
        assert!(names(2024).contains(&("2024-03-20".to_owned(), "春分の日")));
        assert!(names(2023).contains(&("2023-03-21".to_owned(), "春分の日")));
    }

    #[test]
    fn every_year_has_at_least_the_sixteen_named_days() {
        for year in 2024..=2030 {
            assert!(japanese(year).len() >= 16, "{year}: {:?}", japanese(year));
        }
    }
}

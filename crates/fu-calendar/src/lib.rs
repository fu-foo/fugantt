//! Japanese public holidays.
//!
//! Typing sixteen dates a year by hand is the kind of work a schedule tool
//! should not create. The rules are in 国民の祝日に関する法律, and they are
//! rules rather than a list: two of the days move with the equinoxes, four are
//! "the nth Monday", and two more are produced by holidays landing badly.
//!
//! Valid from 2020, when 天皇誕生日 moved to 2月23日 and 体育の日 became
//! スポーツの日. Earlier years would need the older names and dates.
//!
//! This crate depends on nothing. Whoever uses it has already chosen a date
//! library, and it is not this crate's place to bring a second one: the `jiff`
//! and `chrono` features turn a [`Day`] into theirs.

use std::fmt;

/// A calendar day that exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day {
    year: i16,
    month: u8,
    day: u8,
}

impl Day {
    /// The day, or `None` when the calendar has no such day.
    pub fn new(year: i16, month: u8, day: u8) -> Option<Self> {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let length = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => return None,
        };

        (1..=length)
            .contains(&day)
            .then_some(Self { year, month, day })
    }

    pub fn year(self) -> i16 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }

    /// 0 for Monday through 6 for Sunday.
    pub fn weekday(self) -> u8 {
        // The first of January 1970 was a Thursday.
        (self.serial() + 3).rem_euclid(7) as u8
    }

    /// The day after.
    pub fn next(self) -> Self {
        Self::from_serial(self.serial() + 1)
    }

    /// Days since the first of January 1970.
    fn serial(self) -> i64 {
        let (month, day) = (i64::from(self.month), i64::from(self.day));
        // Counted from March, so the leap day is the last of its year.
        let year = i64::from(self.year) - i64::from(month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

        era * 146_097 + day_of_era - 719_468
    }

    fn from_serial(serial: i64) -> Self {
        let shifted = serial + 719_468;
        let era = shifted.div_euclid(146_097);
        let day_of_era = shifted - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let from_march = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * from_march + 2) / 5 + 1;
        let month = if from_march < 10 {
            from_march + 3
        } else {
            from_march - 9
        };
        let year = year_of_era + era * 400 + i64::from(month <= 2);

        Self {
            year: year as i16,
            month: month as u8,
            day: day as u8,
        }
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(feature = "jiff")]
impl From<Day> for jiff::civil::Date {
    fn from(day: Day) -> Self {
        // A `Day` only exists for a day the calendar has.
        jiff::civil::date(day.year, day.month as i8, day.day as i8)
    }
}

#[cfg(feature = "chrono")]
impl From<Day> for chrono::NaiveDate {
    fn from(day: Day) -> Self {
        chrono::NaiveDate::from_ymd_opt(day.year.into(), day.month.into(), day.day.into())
            .expect("a Day is a day the calendar has")
    }
}

const SUNDAY: u8 = 6;

/// Every public holiday in `year`, in date order, with its name.
pub fn japanese(year: i16) -> Vec<(Day, &'static str)> {
    let mut days: Vec<(Day, &'static str)> = Vec::new();

    let mut fixed = |month: u8, day: u8, name: &'static str| {
        if let Some(date) = Day::new(year, month, day) {
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

fn nth_monday(year: i16, month: u8, nth: u8) -> Option<Day> {
    let first = Day::new(year, month, 1)?;

    // Days from the 1st to that month's first Monday.
    let offset = (7 - first.weekday()) % 7;

    Day::new(year, month, 1 + offset + (nth - 1) * 7)
}

enum Season {
    Spring,
    Autumn,
}

/// The equinox days, from the approximation the Cabinet Office publishes.
///
/// Good for 1980–2099.
fn equinox(year: i16, season: Season) -> Option<Day> {
    if !(1980..=2099).contains(&year) {
        return None;
    }

    let base = match season {
        Season::Spring => 20.8431,
        Season::Autumn => 23.2488,
    };

    let years = f64::from(year - 1980);
    let day = (base + 0.242_194 * years - (years / 4.0).floor()).floor() as u8;
    let month = match season {
        Season::Spring => 3,
        Season::Autumn => 9,
    };

    Day::new(year, month, day)
}

/// 振替休日: a holiday on a Sunday moves to the next day that is not itself one.
fn add_substitutes(days: &mut Vec<(Day, &'static str)>) {
    let existing: Vec<Day> = days.iter().map(|(date, _)| *date).collect();
    let mut extra: Vec<(Day, &'static str)> = Vec::new();

    for date in existing.iter().filter(|day| day.weekday() == SUNDAY) {
        let mut candidate = *date;

        // May 3rd lands on a Sunday behind two more holidays, so this walks
        // rather than simply adding a day.
        loop {
            candidate = candidate.next();

            if !existing.contains(&candidate) && !extra.iter().any(|(day, _)| *day == candidate) {
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
fn add_citizens_holidays(days: &mut Vec<(Day, &'static str)>) {
    let existing: Vec<Day> = days.iter().map(|(date, _)| *date).collect();
    let mut extra = Vec::new();

    for date in &existing {
        let gap = date.next();
        let after = gap.next();

        if existing.contains(&after) && !existing.contains(&gap) && gap.weekday() != SUNDAY {
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
            .map(|(day, name)| (day.to_string(), name))
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

        assert!(
            days.contains(&("2026-05-06".to_owned(), "振替休日")),
            "{days:?}"
        );
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

    /// The arithmetic here is this crate's own, so it is held against a
    /// library that has had more eyes on it: every day of eighty years.
    #[test]
    fn the_days_and_weekdays_agree_with_jiff() {
        let mut ours = Day::new(2020, 1, 1).unwrap();
        let mut theirs = jiff::civil::date(2020, 1, 1);

        while theirs.year() < 2100 {
            assert_eq!(ours.to_string(), theirs.to_string());
            assert_eq!(
                i8::try_from(ours.weekday()).unwrap(),
                theirs.weekday().to_monday_zero_offset(),
                "{theirs}"
            );

            ours = ours.next();
            theirs = theirs.tomorrow().unwrap();
        }
    }

    #[test]
    fn a_day_that_does_not_exist_is_not_a_day() {
        assert!(Day::new(2026, 2, 30).is_none());
        assert!(Day::new(2026, 13, 1).is_none());
        assert!(Day::new(2026, 0, 1).is_none());
        assert!(Day::new(2026, 4, 0).is_none());
        assert!(Day::new(2024, 2, 29).is_some());
        assert!(Day::new(2026, 2, 29).is_none());
        assert!(Day::new(2100, 2, 29).is_none());
    }

    /// Outside the years the equinox approximation covers there is still an
    /// answer, and it is short two days rather than wrong or a panic.
    #[test]
    fn a_year_past_the_equinox_table_goes_without_them() {
        for year in [1979, 2100] {
            let days = japanese(year);

            assert!(
                !days.iter().any(|(_, name)| name.ends_with("分の日")),
                "{year}"
            );
            assert!(days.len() >= 14, "{year}: {days:?}");
        }
    }

    #[test]
    fn the_holidays_come_in_date_order() {
        for year in 2020..=2099 {
            let days = japanese(year);
            assert!(days.windows(2).all(|pair| pair[0].0 < pair[1].0), "{year}");
        }
    }
}

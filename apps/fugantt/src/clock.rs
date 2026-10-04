//! What day it is.
//!
//! "Late" is a question about the calendar day of whoever is reading the plan,
//! so every place that asks takes the answer from here, in the server's local
//! zone rather than UTC.

use jiff::civil::Date;

/// Today.
///
/// A debug build reads `FUGANTT_TODAY` first, so a test can compare what the
/// server says against what it said when the answer was written down. A
/// release build has no such branch: the day is the day.
pub fn today() -> Date {
    #[cfg(debug_assertions)]
    if let Some(pinned) = std::env::var("FUGANTT_TODAY")
        .ok()
        .and_then(|day| day.parse().ok())
    {
        return pinned;
    }

    jiff::Zoned::now().date()
}

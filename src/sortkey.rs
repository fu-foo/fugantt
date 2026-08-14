//! Fractional indexing for task order.
//!
//! Rows carry a string key instead of an integer position, so inserting or
//! moving a row is one UPDATE and never renumbers its siblings. The keys sort
//! lexicographically, and there is always room to make a new one between any
//! two existing ones.

const DIGITS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

/// A key that sorts strictly between `before` and `after`.
///
/// `None` means "no bound": `between(None, None)` is the first key in an empty
/// list, and `between(last, None)` appends.
///
/// # Panics
///
/// Panics in debug builds when `before` is not strictly less than `after`.
pub fn between(before: Option<&str>, after: Option<&str>) -> String {
    debug_assert!(
        match (before, after) {
            (Some(before), Some(after)) => before < after,
            _ => true,
        },
        "sort keys must be given in order"
    );

    midpoint(before.unwrap_or(""), after)
}

/// The shortest key strictly between `a` and `b`, where `""` is the low end
/// and `None` the high end.
fn midpoint(a: &str, b: Option<&str>) -> String {
    if let Some(b) = b {
        // A shared prefix cannot move, so carry it over and split what is left.
        let shared = a
            .bytes()
            .chain(std::iter::repeat(DIGITS[0]))
            .zip(b.bytes())
            .take_while(|(a, b)| a == b)
            .count();

        if shared > 0 {
            let rest = a.get(shared..).unwrap_or("");
            return format!("{}{}", &b[..shared], midpoint(rest, Some(&b[shared..])));
        }
    }

    let low = a.bytes().next().map_or(0, index_of);
    let high = match b {
        Some(b) => b.bytes().next().map_or(0, index_of),
        None => DIGITS.len(),
    };

    // Room for a digit between them: one character is enough.
    if high - low > 1 {
        return char::from(DIGITS[(low + high) / 2]).to_string();
    }

    match b {
        // Adjacent digits, but `b` has more to it: borrowing its first digit
        // alone already lands below it.
        Some(b) if b.len() > 1 => b[..1].to_owned(),
        // Otherwise keep `a`'s first digit and find room further right.
        _ => format!(
            "{}{}",
            a.get(..1).unwrap_or("a"),
            midpoint(a.get(1..).unwrap_or(""), None)
        ),
    }
}

fn index_of(digit: u8) -> usize {
    usize::from(digit - DIGITS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_key_sits_in_the_middle_of_the_space() {
        let key = between(None, None);

        assert!(!key.is_empty());
        // Room on both sides is the whole point of not starting at "a".
        assert!(between(None, Some(&key)) < key);
        assert!(between(Some(&key), None) > key);
    }

    #[test]
    fn appending_keeps_ascending() {
        let mut keys: Vec<String> = Vec::new();

        for _ in 0..64 {
            keys.push(between(keys.last().map(String::as_str), None));
        }

        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    /// The case that would break an integer position column: inserting into
    /// the same gap over and over.
    #[test]
    fn a_gap_never_runs_out() {
        let low = between(None, None);
        let mut high = between(Some(&low), None);

        for _ in 0..64 {
            let next = between(Some(&low), Some(&high));

            assert!(low < next, "{low} < {next}");
            assert!(next < high, "{next} < {high}");

            high = next;
        }
    }

    #[test]
    fn prepending_keeps_descending() {
        let mut key = between(None, None);

        for _ in 0..64 {
            let next = between(None, Some(&key));

            assert!(next < key, "{next} < {key}");

            key = next;
        }
    }

    /// Keys with a shared prefix are the ones a naive midpoint gets wrong.
    #[test]
    fn keys_sharing_a_prefix_still_split() {
        let a = "nn";
        let b = "no";

        let mid = between(Some(a), Some(b));

        assert!(a < mid.as_str() && mid.as_str() < b, "{a} < {mid} < {b}");
    }
}

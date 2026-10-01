//! Naught's birthday: the 11th of August, year 1096.

use chrono::{Datelike, Local, NaiveDate};

const BIRTH_YEAR: i32 = 1096;
const BIRTH_MONTH: u32 = 8;
const BIRTH_DAY: u32 = 11;

/// Example: `is_birthday(NaiveDate::from_ymd_opt(2026, 8, 11).unwrap()) == true`.
pub fn is_birthday(date: NaiveDate) -> bool {
    date.month() == BIRTH_MONTH && date.day() == BIRTH_DAY
}

/// The age Naught turns in `date`'s year.
pub fn turning_age(date: NaiveDate) -> i32 {
    date.year() - BIRTH_YEAR
}

/// Today in the local time zone.
pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn only_august_eleventh() {
        assert!(is_birthday(d(2026, 8, 11)));
        assert!(!is_birthday(d(2026, 11, 8)));
        assert!(!is_birthday(d(2026, 8, 12)));
    }

    #[test]
    fn turns_930_in_2026() {
        assert_eq!(turning_age(d(2026, 8, 11)), 930);
        assert_eq!(turning_age(d(2027, 1, 1)), 931);
    }
}

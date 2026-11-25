//! Timestamp parsing and formatting for OKF trust fields.
//!
//! OKF timestamps are RFC 3339 date-times, and a bare date means the
//! start of that day in UTC. Parsing and formatting are implemented
//! directly on the standard library so `perishables-core` stays free
//! of external dependencies. Any value outside exact Gregorian
//! calendar bounds or RFC 3339 structure returns `None`.

const SECONDS_PER_DAY: i64 = 86_400;

/// Whether `year` is a proleptic Gregorian leap year.
fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Exact day count for a Gregorian `(year, month)` pair.
fn days_in_month(year: i64, month: i64) -> Option<i64> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(if is_leap_year(year) { 29 } else { 28 }),
        _ => None,
    }
}

/// Days from the civil date, after Howard Hinnant's algorithm.
/// Returns days since `1970-01-01`.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Civil `(year, month, day)` in UTC from days since `1970-01-01`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

fn fixed_digits(text: &str, expected_len: usize) -> Option<i64> {
    if text.len() != expected_len || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

/// Parse an OKF timestamp into seconds since the Unix epoch in UTC.
///
/// Accepts a bare date (`2026-11-20`) or an RFC 3339 date-time with
/// a `Z`/`z` suffix or a numeric `±HH:MM` offset. Returns `None` when
/// the text is not a valid calendar date or RFC 3339 timestamp.
pub fn parse_timestamp(text: &str) -> Option<i64> {
    let s = text.trim().trim_matches('"').trim_matches('\'');
    let (date_part, time_part) = match s.split_once(['T', 't', ' ']) {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let mut date = date_part.split('-');
    let year = fixed_digits(date.next()?, 4)?;
    let month = fixed_digits(date.next()?, 2)?;
    let day = fixed_digits(date.next()?, 2)?;
    if date.next().is_some() {
        return None;
    }
    let max_day = days_in_month(year, month)?;
    if !(1..=max_day).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let Some(time) = time_part else {
        return Some(days * SECONDS_PER_DAY);
    };
    if time.is_empty() {
        return None;
    }
    let (hms, offset_seconds) = if let Some(rest) = time.strip_suffix(['Z', 'z']) {
        (rest, 0)
    } else {
        let idx = time.rfind(['+', '-'])?;
        if idx == 0 {
            return None;
        }
        let (hms, off) = time.split_at(idx);
        let sign = if off.as_bytes()[0] == b'+' { 1 } else { -1 };
        let mut off_parts = off[1..].split(':');
        let oh = fixed_digits(off_parts.next()?, 2)?;
        let om = fixed_digits(off_parts.next()?, 2)?;
        if off_parts.next().is_some() || oh > 23 || om > 59 {
            return None;
        }
        (hms, sign * (oh * 3600 + om * 60))
    };
    let mut parts = hms.split(':');
    let hour = fixed_digits(parts.next()?, 2)?;
    let minute = fixed_digits(parts.next()?, 2)?;
    let sec_raw = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let second = match sec_raw.split_once('.') {
        Some((whole, frac)) => {
            if frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            fixed_digits(whole, 2)?
        }
        None => fixed_digits(sec_raw, 2)?,
    };
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(days * SECONDS_PER_DAY + hour * 3600 + minute * 60 + second - offset_seconds)
}

/// Format Unix epoch seconds as a canonical UTC RFC 3339 timestamp.
pub fn format_rfc3339(epoch_seconds: i64) -> String {
    let days = epoch_seconds.div_euclid(SECONDS_PER_DAY);
    let rem = epoch_seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

#[cfg(test)]
mod tests {
    use super::{format_rfc3339, parse_timestamp};

    #[test]
    fn epoch_and_dates() {
        assert_eq!(parse_timestamp("1970-01-01"), Some(0));
        assert_eq!(parse_timestamp("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_timestamp("1970-01-01t00:00:00z"), Some(0));
        assert_eq!(parse_timestamp("2026-11-20"), Some(1_795_132_800));
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_rfc3339(1_795_132_800), "2026-11-20T00:00:00Z");
    }

    #[test]
    fn offsets_normalize_to_utc() {
        assert_eq!(
            parse_timestamp("2026-06-25T11:00:00+02:00"),
            parse_timestamp("2026-06-25T09:00:00Z")
        );
        assert_eq!(
            parse_timestamp("2026-06-25T04:00:00-05:00"),
            parse_timestamp("2026-06-25T09:00:00Z")
        );
        assert_eq!(
            parse_timestamp("2026-06-25T09:00:00.123456Z"),
            parse_timestamp("2026-06-25T09:00:00Z")
        );
    }

    #[test]
    fn calendar_and_rfc3339_bounds_are_enforced() {
        assert_eq!(parse_timestamp("next spring"), None);
        assert_eq!(parse_timestamp(""), None);
        assert_eq!(parse_timestamp("2026-13-01"), None);
        assert_eq!(parse_timestamp("2026-02-29"), None);
        assert!(parse_timestamp("2024-02-29").is_some());
        assert_eq!(parse_timestamp("1900-02-29"), None);
        assert!(parse_timestamp("2000-02-29").is_some());
        assert_eq!(parse_timestamp("2026-04-31"), None);
        assert_eq!(parse_timestamp("2026-11-20T10:00:00.abcZ"), None);
        assert_eq!(parse_timestamp("2026-11-20T10:00:00.Z"), None);
        assert_eq!(parse_timestamp("2026-11-20T10:00:00:00Z"), None);
        assert_eq!(parse_timestamp("2026-11-20T10:00:00+24:00"), None);
        assert_eq!(parse_timestamp("2026-11-20T10:00:00+02:60"), None);
        assert_eq!(parse_timestamp("2026-11-20T10:00:00+02"), None);
        assert_eq!(parse_timestamp("2026-11-20T10:00:00"), None);
    }
}

//! Timestamp parsing for OKF trust fields.
//!
//! OKF timestamps are RFC 3339 date-times, and a bare date means the
//! start of that day in UTC. Parsing is done by hand so the core
//! crate stays free of external dependencies. A value that does not
//! parse is reported as unparseable by the caller, never guessed.

const SECONDS_PER_DAY: i64 = 86_400;

/// Days from the civil date, after Howard Hinnant's algorithm.
/// Returns days since 1970-01-01.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn digits(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

/// Parse an OKF timestamp into seconds since the Unix epoch in UTC.
///
/// Accepts a bare date (`2026-11-20`) or an RFC 3339 date-time with
/// a `Z` suffix or a numeric offset. Returns `None` when the value
/// is not a timestamp.
pub fn parse_timestamp(text: &str) -> Option<i64> {
    let s = text.trim().trim_matches('"').trim_matches('\'');
    let (date_part, time_part) = match s.split_once(['T', ' ']) {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let mut date = date_part.split('-');
    let year = digits(date.next()?)?;
    let month = digits(date.next()?)?;
    let day = digits(date.next()?)?;
    if date.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let Some(time) = time_part else {
        return Some(days * SECONDS_PER_DAY);
    };
    let (hms, offset_seconds) = if let Some(rest) = time.strip_suffix('Z') {
        (rest, 0)
    } else if let Some(idx) = time.rfind(['+', '-']) {
        if idx == 0 {
            return None;
        }
        let (hms, off) = time.split_at(idx);
        let sign = if time.as_bytes()[idx] == b'+' { 1 } else { -1 };
        let mut parts = off[1..].split(':');
        let oh = digits(parts.next()?)?;
        let om = digits(parts.next().unwrap_or("0"))?;
        (hms, sign * (oh * 3600 + om * 60))
    } else {
        (time, 0)
    };
    let mut parts = hms.split(':');
    let hour = digits(parts.next()?)?;
    let minute = digits(parts.next()?)?;
    let second = match parts.next() {
        Some(sec) => digits(sec.split('.').next()?)?,
        None => 0,
    };
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(days * SECONDS_PER_DAY + hour * 3600 + minute * 60 + second - offset_seconds)
}

#[cfg(test)]
mod tests {
    use super::parse_timestamp;

    #[test]
    fn epoch_and_dates() {
        assert_eq!(parse_timestamp("1970-01-01"), Some(0));
        assert_eq!(parse_timestamp("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_timestamp("2026-11-20"), Some(1_795_132_800));
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
    }

    #[test]
    fn prose_is_not_a_timestamp() {
        assert_eq!(parse_timestamp("next spring"), None);
        assert_eq!(parse_timestamp(""), None);
        assert_eq!(parse_timestamp("2026-13-01"), None);
    }
}

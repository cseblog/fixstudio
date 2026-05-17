//! Slim utility module — Health feature was removed from the UI; only the
//! shared FIX timestamp parser remains, reused by `fill_quality` and
//! `session_summary` for latency calculations.

/// Parse a FIX `SendingTime`-style timestamp ("YYYYMMDD-HH:MM:SS[.fraction]"
/// or "YYYY-MM-DD HH:MM:SS[.fraction]") into microseconds since midnight.
pub fn parse_time_us(s: &str) -> Option<i64> {
    let time_part: &str = if let Some(sp) = s.find(' ') {
        &s[sp + 1..]
    } else if let Some(dash) = s.find('-') {
        &s[dash + 1..]
    } else {
        return None;
    };
    let (hms, frac_opt) = match time_part.find('.') {
        Some(dot) => (&time_part[..dot], Some(&time_part[dot + 1..])),
        None      => (time_part, None),
    };
    let mut parts = hms.split(':');
    let h: i64   = parts.next()?.parse().ok()?;
    let m: i64   = parts.next()?.parse().ok()?;
    let sec: i64 = parts.next()?.parse().ok()?;
    let mut us   = (h * 3_600 + m * 60 + sec) * 1_000_000;
    if let Some(frac) = frac_opt {
        let flen   = frac.len().min(6);
        let fval: i64 = frac[..flen].parse().unwrap_or(0);
        us += fval * 10i64.pow((6 - flen) as u32);
    }
    Some(us)
}

#[cfg(test)]
mod tests {
    use super::parse_time_us;

    #[test]
    fn parses_fix_style_dash_separator() {
        // FIX standard: YYYYMMDD-HH:MM:SS.fff
        let us = parse_time_us("20240315-14:18:24.985282").unwrap();
        let expected =
            (14 * 3600 + 18 * 60 + 24) as i64 * 1_000_000 + 985_282;
        assert_eq!(us, expected);
    }

    #[test]
    fn parses_space_separator() {
        let us = parse_time_us("2024-03-15 14:18:24.985").unwrap();
        let expected =
            (14 * 3600 + 18 * 60 + 24) as i64 * 1_000_000 + 985_000;
        assert_eq!(us, expected);
    }

    #[test]
    fn parses_without_fractional_seconds() {
        let us = parse_time_us("20240315-00:00:01").unwrap();
        assert_eq!(us, 1_000_000);
    }

    #[test]
    fn truncates_fractional_beyond_six_digits() {
        // 7 digits → only first 6 contribute (nanoseconds dropped).
        let us = parse_time_us("20240101-00:00:00.1234567").unwrap();
        assert_eq!(us, 123_456);
    }

    #[test]
    fn pads_short_fractional_to_microseconds() {
        // ".5" → 500_000us, not 5us.
        let us = parse_time_us("20240101-00:00:00.5").unwrap();
        assert_eq!(us, 500_000);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_time_us("not a timestamp").is_none());
        assert!(parse_time_us("").is_none());
    }
}

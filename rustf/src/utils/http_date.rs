//! HTTP date formatting and parsing for response headers.
//!
//! Centralizes the `Last-Modified` / `If-Modified-Since` / `Date` handling
//! that was previously duplicated across `app.rs`, `cache/response.rs`, and
//! `security/static_files.rs` (the last of which was still a placeholder).
//!
//! The formatter is hand-rolled against the fixed RFC 7231 preferred shape
//! (`"Sun, 06 Nov 1994 08:49:37 GMT"` — 29 bytes) to avoid the overhead of
//! `chrono::DateTime::format` parsing a strftime string on every call. The
//! parser accepts both RFC 7231 and the legacy RFC 850 form (`"Sunday,
//! 06-Nov-94 08:49:37 GMT"`) per RFC 7231 §7.1.1.1.

use chrono::{Datelike, TimeZone, Timelike, Utc};
use std::fmt::Write;

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Format a Unix timestamp (seconds since epoch) as an RFC 7231 HTTP date.
///
/// Output is always exactly 29 bytes: `"Sun, 06 Nov 1994 08:49:37 GMT"`.
/// Out-of-range timestamps fall back to the Unix epoch.
pub fn format_http_date(timestamp: u64) -> String {
    let dt = Utc
        .timestamp_opt(timestamp as i64, 0)
        .single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());

    let day_name = DAYS[dt.weekday().num_days_from_sunday() as usize];
    let month_name = MONTHS[(dt.month() - 1) as usize];

    let mut s = String::with_capacity(29);
    // write! to String is infallible — unwrap is a no-op we still keep
    // silent via let _.
    let _ = write!(
        s,
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        day_name,
        dt.day(),
        month_name,
        dt.year(),
        dt.hour(),
        dt.minute(),
        dt.second(),
    );
    s
}

/// Parse an HTTP date header value into a Unix timestamp.
///
/// RFC 7231 §7.1.1.1 requires recipients to accept all three historical
/// formats:
///   1. IMF-fixdate / RFC 7231:  `Sun, 06 Nov 1994 08:49:37 GMT`
///   2. RFC 850 obsolete:        `Sunday, 06-Nov-94 08:49:37 GMT`
///   3. ANSI C asctime():        `Sun Nov  6 08:49:37 1994`
///
/// Returns `None` for unrecognised or negative values.
pub fn parse_http_date(date_str: &str) -> Option<u64> {
    // Formats 1 and 2 both end in " GMT" — strip it once, try both.
    if let Some(without_gmt) = date_str.strip_suffix(" GMT") {
        if let Ok(naive) =
            chrono::NaiveDateTime::parse_from_str(without_gmt, "%a, %d %b %Y %H:%M:%S")
        {
            let ts = Utc.from_utc_datetime(&naive).timestamp();
            if ts >= 0 {
                return Some(ts as u64);
            }
        }
        if let Ok(naive) =
            chrono::NaiveDateTime::parse_from_str(without_gmt, "%A, %d-%b-%y %H:%M:%S")
        {
            let ts = Utc.from_utc_datetime(&naive).timestamp();
            if ts >= 0 {
                return Some(ts as u64);
            }
        }
    }

    // asctime has no " GMT" suffix — parse against the raw string.
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(date_str, "%a %b %e %H:%M:%S %Y") {
        let ts = Utc.from_utc_datetime(&naive).timestamp();
        if ts >= 0 {
            return Some(ts as u64);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_rfc_7231_shape() {
        // 1994-11-06 08:49:37 UTC → the RFC 7231 canonical example.
        let ts = 784111777u64;
        assert_eq!(format_http_date(ts), "Sun, 06 Nov 1994 08:49:37 GMT");
    }

    #[test]
    fn formats_epoch() {
        assert_eq!(format_http_date(0), "Thu, 01 Jan 1970 00:00:00 GMT");
    }

    #[test]
    fn formats_fixed_width_29_bytes() {
        // Every valid output must be exactly 29 ASCII bytes.
        for ts in [0u64, 1, 1_000_000, 784_111_777, 2_000_000_000] {
            let s = format_http_date(ts);
            assert_eq!(s.len(), 29, "wrong length for ts={}: {:?}", ts, s);
        }
    }

    #[test]
    fn parses_rfc_7231() {
        assert_eq!(
            parse_http_date("Sun, 06 Nov 1994 08:49:37 GMT"),
            Some(784111777)
        );
    }

    #[test]
    fn parses_rfc_850() {
        assert_eq!(
            parse_http_date("Sunday, 06-Nov-94 08:49:37 GMT"),
            Some(784111777)
        );
    }

    #[test]
    fn parses_asctime() {
        assert_eq!(
            parse_http_date("Sun Nov  6 08:49:37 1994"),
            Some(784111777)
        );
    }

    #[test]
    fn rejects_bogus() {
        assert_eq!(parse_http_date("not a date"), None);
        assert_eq!(parse_http_date("Sun, 06 Nov 1994 08:49:37"), None); // missing GMT
    }

    #[test]
    fn round_trip() {
        for ts in [0u64, 784_111_777, 1_700_000_000] {
            let s = format_http_date(ts);
            assert_eq!(parse_http_date(&s), Some(ts));
        }
    }
}
